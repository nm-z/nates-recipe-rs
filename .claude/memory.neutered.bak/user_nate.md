---
name: Nate's profile
description: User background, hardware, and working style for the nates-recipe-rs AutoML project
type: user
originSessionId: 36551121-db6d-49ea-896e-8ae13f19d2f0
---
Experienced developer comfortable with Rust, systems programming, GPU compute, ML/AutoML pipelines. Has deep knowledge of Kaggle competition workflows and sklearn-style ML. Previously built the Python version (nates_recipe-V2) using Optuna TPE, sklearn, PyTorch.

Hardware: AMD Ryzen 5 7600X (6c/12t), AMD RX 7700/7800 XT (RDNA 3, 12GB VRAM), Arch Linux. ROCm/HIP installed at /opt/rocm (symlinked to /home/nate/.rocm-install/rocm due to root partition space). No CUDA.

Disk layout: root partition (nvme0n1p3, 92GB) is tight — ROCm fills most of it. Home partition (nvme0n1p2, 820GB) has plenty of space. Rust build targets can be huge (16GB+).
