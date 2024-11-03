FROM rust:1.82 AS builder
WORKDIR /usr/src/proj
COPY . .
WORKDIR /usr/src/proj/apps/server
RUN cargo install --path .

FROM ubuntu:rolling
COPY --from=builder /usr/local/cargo/bin/server /usr/local/bin/server
EXPOSE 3000
CMD ["server"]