FROM ubuntu:latest

RUN apt-get update
RUN apt-get install -y \
    curl \
    strace

COPY nexus /usr/local/bin
