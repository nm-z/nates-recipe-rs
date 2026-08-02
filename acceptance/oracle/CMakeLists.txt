cmake_minimum_required(VERSION 3.24)
project(recipe-llama-oracle LANGUAGES C CXX)

set(LLAMA_CPP_REVISION "aedb2a5e9ca3d4064148bbb919e0ddc0c1b70ab3")
set(LLAMA_CPP_ROOT "" CACHE PATH "Exact pinned llama.cpp checkout")
if(NOT EXISTS "${LLAMA_CPP_ROOT}/include/llama.h")
    message(FATAL_ERROR "LLAMA_CPP_ROOT must name a llama.cpp checkout")
endif()

execute_process(
    COMMAND git rev-parse HEAD
    WORKING_DIRECTORY "${LLAMA_CPP_ROOT}"
    OUTPUT_VARIABLE LLAMA_CPP_HEAD
    OUTPUT_STRIP_TRAILING_WHITESPACE
    COMMAND_ERROR_IS_FATAL ANY
)
if(NOT LLAMA_CPP_HEAD STREQUAL LLAMA_CPP_REVISION)
    message(FATAL_ERROR "llama.cpp must be exactly ${LLAMA_CPP_REVISION}; found ${LLAMA_CPP_HEAD}")
endif()

set(BUILD_SHARED_LIBS OFF CACHE BOOL "" FORCE)
set(GGML_CUDA ON CACHE BOOL "" FORCE)
set(GGML_CUDA_GRAPHS ON CACHE BOOL "" FORCE)
set(LLAMA_BUILD_COMMON OFF CACHE BOOL "" FORCE)
set(LLAMA_BUILD_TESTS OFF CACHE BOOL "" FORCE)
set(LLAMA_BUILD_TOOLS OFF CACHE BOOL "" FORCE)
set(LLAMA_BUILD_EXAMPLES OFF CACHE BOOL "" FORCE)
set(LLAMA_BUILD_SERVER OFF CACHE BOOL "" FORCE)
set(LLAMA_BUILD_APP OFF CACHE BOOL "" FORCE)
set(LLAMA_BUILD_UI OFF CACHE BOOL "" FORCE)
add_subdirectory("${LLAMA_CPP_ROOT}" llama.cpp EXCLUDE_FROM_ALL)

add_executable(recipe-llama-oracle main.cpp)
target_compile_features(recipe-llama-oracle PRIVATE cxx_std_17)
target_compile_definitions(recipe-llama-oracle PRIVATE RECIPE_LLAMA_CPP_REVISION="${LLAMA_CPP_REVISION}")
target_link_libraries(recipe-llama-oracle PRIVATE llama)
