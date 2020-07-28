# TODO: Dedupe these commands from the Makefile.
# (Or just delete the makefile and run in docker?)

############### RUST STUFF #################
FROM rust:1.45-slim as web
WORKDIR /bobbin-web

# First, copy and build just our dependencies, with a dummy main. This way,
# future builds will reuse these dependencies.
COPY ./web/Cargo.toml .
COPY ./web/Cargo.lock .

RUN ["mkdir", "src/"]
RUN echo 'fn main() {panic!("if you see this, the build broke")}' > src/main.rs
RUN ["cargo", "build", "--release"]

# Now that dependencies are built, rebuild with the actual source. The previous
# steps will be skipped if Cargo.toml and Cargo.lock didn't chage (which means
# our compiled dependencies will be reused)
RUN rm target/*/deps/bobbin-*
COPY ./web .
RUN ["cargo", "build", "--release"]

############### STATIC STUFF #################
FROM node:10-slim as frontend
WORKDIR /bobbin-frontend
COPY ./frontend .
RUN ["yarn", "install"]
RUN ["yarn", "run", "webpack", "--prod"]
RUN ["yarn", "run", "css-build"]
RUN mkdir static/webfonts && cp node_modules/@fortawesome/fontawesome-free/webfonts/fa-solid* static/webfonts/

# TODO: figure out how to correctly use baseimage
# TODO: or use dumbinit
# FROM phusion/baseimage:bionic-1.0.0
FROM debian:stable-slim
WORKDIR /bobbin
COPY --from=web /bobbin-web/target/release/bobbin .
COPY --from=frontend /bobbin-frontend/static ./static

EXPOSE 8000/tcp
CMD ./bobbin
