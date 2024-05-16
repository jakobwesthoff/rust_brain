FROM --platform=linux/amd64 rust:latest

RUN apt-get update && \
  apt-get install -y neovim tmux nasm gcc gdb

COPY bashrc /root/.bashrc
COPY vimrc /root/.config/nvim/init.vim

WORKDIR /app
