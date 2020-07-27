# TODO: Dedupe these commands from the Makefile.
# (Or just delete the makefile and run in docker?)

FROM rust:1.45-slim as web
WORKDIR /bobbin-web
COPY ./web .
RUN ["cargo" "build" "--release"]

FROM node:10-slim as frontend
WORKDIR /bobbin-frontend
COPY ./frontend .
RUN ["yarn", "install"]
RUN ["yarn", "run", "webpack", "--prod"]
RUN ["yarn", "run", "css-build"]
RUN rsync node_modules/@fortawesome/fontawesome-free/webfonts/fa-solid* static/webfonts/

FROM phusion/baseimage:bionic-1.0.0
WORKDIR /bobbin
COPY --from=web /bobbin-web/target/release/bobbin .
COPY --from=frontend /bobbin-frontend/static .
CMD ["ls", "-la"]
