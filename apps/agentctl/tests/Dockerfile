# Docker image for isolated tmux integration tests.
#
# This image is used by TmuxEnv in tests/e2e/support/tmux-env.ts. It provides a
# minimal environment with tmux pre-installed so that tests never perform
# runtime package installation (which requires container internet access and
# is fragile on macOS Docker Desktop).
#
# The tmux client on the host talks to the tmux server in this container over
# a Unix socket, and tmux refuses to talk across protocol versions. The base
# image is therefore Ubuntu 24.04 so its tmux matches the one on GitHub's
# `ubuntu-latest` runners (24.04).
#
# The entrypoint is overridden at test startup; this CMD is never used
# directly by TmuxEnv but serves as documentation of the intended use.
FROM ubuntu:24.04

RUN apt-get update \
    && apt-get install -y --no-install-recommends tmux bash \
    && rm -rf /var/lib/apt/lists/*

ENV TERM=xterm-256color
