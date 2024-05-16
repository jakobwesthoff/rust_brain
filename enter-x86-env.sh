#!/bin/sh

docker build -t rust_jit_experiments_x86_env x86_env/ && \
docker run -it --rm --cap-add=SYS_PTRACE --security-opt seccomp=unconfined -v ./:/app --platform linux/amd64 rust_jit_experiments_x86_env
