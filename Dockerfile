#################
## build stage ##
#################
FROM messense/rust-musl-cross:x86_64-musl as builder

WORKDIR /code

# Download crates-io index and fetch dependency code.
# This step avoids needing to spend time on every build downloading the index
# which can take a long time within the docker context. Docker will cache it.
RUN USER=root cargo init
COPY Cargo.toml Cargo.toml
RUN cargo fetch

# copy app files
COPY src src

# copy static files
COPY static static

# compile app
RUN cargo build --release

## run stage ##
###############
FROM docker.io/rust:latest
WORKDIR /app

# copy server binary from build stage
COPY --from=builder /code/target/x86_64-unknown-linux-musl/release/project project

# copy static files
COPY --from=builder /code/static static

# indicate what port the server is running on
EXPOSE 8080

# run server
CMD [ "/app/project" ]