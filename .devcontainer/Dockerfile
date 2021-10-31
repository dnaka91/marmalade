FROM mcr.microsoft.com/vscode/devcontainers/base:ubuntu-20.04

RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
    && apt-get -y install --no-install-recommends build-essential fish libssl-dev lld pkg-config

USER vscode
COPY ./bootstrap.sh /tmp/bootstrap.sh
RUN /tmp/bootstrap.sh
