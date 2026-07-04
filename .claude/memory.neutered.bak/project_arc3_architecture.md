---
name: project_arc3_architecture
description: "ARC-AGI-3 Kaggle agent (arc3 crate) — gateway harness + v1 MANN, evolving to an LSTM-meta-learner / MANN feature-selection-bottleneck design"
metadata: 
  node_type: memory
  type: project
  originSessionId: 0bcacdad-2dc0-44fd-b258-36893e624d86
---

`arc3/` crate (own `[workspace]`, path-deps `gpu-core`). Competition: Kaggle **ARC-AGI-3** (interactive RL benchmark; blind 110 private games, 55 public-LB / 55 private-LB; the 25 public games are dev-only, NOT in the eval set; score per level = `(human_baseline_actions / agent_actions)^2 * 100` capped 115, averaged over a game's levels weighted by level index, then averaged over games; field clustered ~0.6–1.2%).

**Harness (resolved):** engine is a Flask gateway at `http://gateway:8001` in competition mode — agent runs `OPERATION_MODE=online`, gateway scores server-side, the Kaggle notebook is just a launcher that exec's the Rust binary (internet "disabled" still allows the internal `gateway` host). REST: `GET /api/games`; `POST /api/scorecard/{open,close}`; `POST /api/cmd/{RESET,ACTION1..7}` body `{game_id,guid,x,y}`; first RESET (no guid) returns a guid. Frame = stacked 64×64 layers, values 0–15; `available_actions` per frame; ACTION6 carries (x,y)∈0–63. `baseline_actions` are stripped from API responses (hidden denominators). Local dev gateway: install `arc_agi`+`arcengine` wheels, run `Arcade(operation_mode=OFFLINE, environments_dir=…).listen_and_serve(port=8001)`; metadata.json needs a `class_name` added. Per-action frame delta is tiny (~1 cell) → encoder must be positionally sensitive; random play scores 0.

**v1 MANN (built, runs, score-0 baseline):** frozen positional-hash encoder (each (layer,row,col,value) hashed to a signed D-bucket, L2-norm) → external KV memory (write `(state_key,action)->(Δcells,Δlevels,outcome)` each step) → cosine top-k softmax read → `action = argmax(w_prog*progress - w_dang*danger + w_nov*novelty)`. No backprop. Files: `gateway.rs types.rs encoder.rs memory.rs policy.rs agent.rs main.rs`. CPU reference; hot ops slated for gpu-core.

**AUTHORITATIVE spec (learning-to-learn; supersedes the earlier inverted note AND the v1 hash-MANN + world.rs sprite-planner):**
- **LSTM = the LEARNER / action policy.** Inputs: encoded frame + prev action + prev reward/progress + prev terminal + prev hidden. Outputs: action logits/probs (+ optional value/confidence) + hidden. Trained in an INNER loop of a FIXED update budget (e.g. 5/10/20 updates).
- **MANN = the META-LEARNER / teacher; does NOT act.** Observes the LSTM's learning context (frame enc, hidden summary, action output, loss, reward, gradient summary, update-step idx, remaining budget, episode summary, external-memory reads) and OUTPUTS the LSTM's update controls: lr, optimizer coeffs, grad scale, grad clip, loss weighting, exploration temp, hidden-reset gate, memory-write gate, update/no-update, optional init correction. Has external NTM/DNC memory. Trained in the OUTER loop on POST-ADAPTATION LSTM performance (few-shot convergence speed, stability, low forgetting).
- Order: train LSTM standalone (Stage 1) → FREEZE the LSTM arch/contract (input shape, hidden size, #layers, action format, loss/grad interface, param layout, update budget) (Stage 2) → train MANN on that fixed LSTM (Stage 3). Changing the LSTM arch forces MANN retraining.
- Frame encoding is CATEGORICAL (color IDs ≠ continuous): uint8 [layers,H,W] or one-hot [layers,colors,H,W] or learned color embeddings; preserve color/position/layer/temporal-order/available-actions.
- REQUIRED artifacts: lstm_weights.safetensors, mann_weights.safetensors, encoder_weights.safetensors, training_config.toml, run_log.jsonl, checkpoint.bin.
- Valid ONLY if the MANN measurably improves the LSTM's few-shot adaptation within the fixed budget. Built via gpu-core (gpu_lstm_cell_f32, optimizers, losses, attention for MANN memory).

gpu-core fit: `nn_f32::gpu_lstm_cell_f32` (meta-learner cell), `linalg::gpu_dgemv` + `encoding::gpu_pairwise_cosine` (encoder/retrieval), `linalg::gpu_dger`/`gpu_potrs` (added-transform fast-weights / closed-form fits). Related: [[project_gpu_status]], [[project_kaggle_s6e4]]. User wants it done in their Rust/GPU stack, NOT the official Python LangGraph agents.
