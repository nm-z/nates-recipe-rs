#include "ggml-backend.h"
#include "llama.h"

#include <chrono>
#include <cstdint>
#include <cstring>
#include <fstream>
#include <iostream>
#include <limits>
#include <string>
#include <vector>

namespace {

constexpr std::int32_t expected_positions = 128;
constexpr std::int32_t expected_vocabulary = 128;

struct arguments {
    std::string model;
    std::string tokens;
    std::string output;
};

int fail(const std::string & detail) {
    std::cerr << "recipe llama.cpp oracle: " << detail << '\n';
    return 1;
}

bool parse_arguments(int argc, char ** argv, arguments & parsed) {
    for (int index = 1; index < argc; index += 2) {
        if (index + 1 >= argc) {
            return false;
        }
        const std::string flag = argv[index];
        const std::string value = argv[index + 1];
        if (flag == "--model") {
            parsed.model = value;
        } else if (flag == "--tokens") {
            parsed.tokens = value;
        } else if (flag == "--output") {
            parsed.output = value;
        } else {
            return false;
        }
    }
    return !parsed.model.empty() && !parsed.tokens.empty() && !parsed.output.empty();
}

bool read_tokens(const std::string & path, std::vector<llama_token> & tokens) {
    std::ifstream input(path);
    std::int64_t token = 0;
    while (input >> token) {
        if (token < std::numeric_limits<llama_token>::min() || token > std::numeric_limits<llama_token>::max()) {
            return false;
        }
        tokens.push_back(static_cast<llama_token>(token));
    }
    return input.eof() && tokens.size() == static_cast<std::size_t>(expected_positions);
}

void fill_batch(llama_batch & batch, const std::vector<llama_token> & tokens) {
    batch.n_tokens = static_cast<std::int32_t>(tokens.size());
    for (std::int32_t index = 0; index < batch.n_tokens; ++index) {
        batch.token[index] = tokens[static_cast<std::size_t>(index)];
        batch.pos[index] = index;
        batch.n_seq_id[index] = 1;
        batch.seq_id[index][0] = 0;
        batch.logits[index] = 1;
    }
}

bool decode_all(llama_context * context, llama_batch & batch, std::vector<float> & logits) {
    if (llama_decode(context, batch) != 0) {
        return false;
    }
    logits.resize(static_cast<std::size_t>(expected_positions) * static_cast<std::size_t>(expected_vocabulary));
    for (std::int32_t position = 0; position < expected_positions; ++position) {
        const float * row = llama_get_logits_ith(context, position);
        if (row == nullptr) {
            return false;
        }
        std::memcpy(
            logits.data() + static_cast<std::size_t>(position) * static_cast<std::size_t>(expected_vocabulary),
            row,
            static_cast<std::size_t>(expected_vocabulary) * sizeof(float));
    }
    return true;
}

void write_u32(std::ofstream & output, std::uint32_t value) {
    const char bytes[4] = {
        static_cast<char>(value & 0xffU),
        static_cast<char>((value >> 8U) & 0xffU),
        static_cast<char>((value >> 16U) & 0xffU),
        static_cast<char>((value >> 24U) & 0xffU),
    };
    output.write(bytes, 4);
}

bool write_logits(const std::string & path, const std::vector<float> & logits) {
    std::ofstream output(path, std::ios::binary | std::ios::trunc);
    output.write("LGT0", 4);
    write_u32(output, expected_positions);
    write_u32(output, expected_vocabulary);
    for (float value : logits) {
        std::uint32_t bits = 0;
        std::memcpy(&bits, &value, sizeof(bits));
        write_u32(output, bits);
    }
    output.flush();
    return output.good();
}

} // namespace

int main(int argc, char ** argv) {
    if (argc == 2 && std::string(argv[1]) == "--revision") {
        std::cout << RECIPE_LLAMA_CPP_REVISION << '\n';
        return 0;
    }
    arguments parsed;
    if (!parse_arguments(argc, argv, parsed)) {
        return fail("usage: recipe-llama-oracle --model MODEL --tokens TOKENS --output LOGITS");
    }
    std::vector<llama_token> tokens;
    if (!read_tokens(parsed.tokens, tokens)) {
        return fail("token input must contain exactly 128 valid integer token IDs");
    }

    ggml_backend_load_all();
    std::size_t gpu_count = 0;
    for (std::size_t index = 0; index < ggml_backend_dev_count(); ++index) {
        if (ggml_backend_dev_type(ggml_backend_dev_get(index)) == GGML_BACKEND_DEVICE_TYPE_GPU) {
            ++gpu_count;
        }
    }
    if (!llama_supports_gpu_offload() || gpu_count != 1) {
        return fail("the pinned oracle requires exactly one live GPU backend");
    }

    llama_model_params model_parameters = llama_model_default_params();
    model_parameters.n_gpu_layers = -1;
    llama_model * model = llama_model_load_from_file(parsed.model.c_str(), model_parameters);
    if (model == nullptr) {
        return fail("failed to load the GGUF model");
    }
    const llama_vocab * vocabulary = llama_model_get_vocab(model);
    if (vocabulary == nullptr || llama_vocab_n_tokens(vocabulary) != expected_vocabulary) {
        llama_model_free(model);
        return fail("GGUF vocabulary is not the required 128-token instrument");
    }

    llama_context_params context_parameters = llama_context_default_params();
    context_parameters.n_ctx = expected_positions;
    context_parameters.n_batch = expected_positions;
    context_parameters.n_ubatch = expected_positions;
    context_parameters.no_perf = false;
    llama_context * context = llama_init_from_model(model, context_parameters);
    if (context == nullptr) {
        llama_model_free(model);
        return fail("failed to create the GPU inference context");
    }
    llama_batch batch = llama_batch_init(expected_positions, 0, 1);
    fill_batch(batch, tokens);

    std::vector<float> warmup;
    if (!decode_all(context, batch, warmup)) {
        llama_batch_free(batch);
        llama_free(context);
        llama_model_free(model);
        return fail("untimed GPU warm-up decode failed");
    }
    llama_memory_clear(llama_get_memory(context), true);
    fill_batch(batch, tokens);

    std::vector<float> logits;
    const auto started = std::chrono::steady_clock::now();
    const bool decoded = decode_all(context, batch, logits);
    const auto elapsed = std::chrono::steady_clock::now() - started;
    if (!decoded) {
        llama_batch_free(batch);
        llama_free(context);
        llama_model_free(model);
        return fail("measured GPU decode failed");
    }
    const auto elapsed_ns = std::chrono::duration_cast<std::chrono::nanoseconds>(elapsed).count();
    if (elapsed_ns <= 0 || !write_logits(parsed.output, logits)) {
        llama_batch_free(batch);
        llama_free(context);
        llama_model_free(model);
        return fail("failed to write measured logits");
    }

    llama_batch_free(batch);
    llama_free(context);
    llama_model_free(model);
    std::cout << "elapsed_ns=" << elapsed_ns << '\n';
    return 0;
}
