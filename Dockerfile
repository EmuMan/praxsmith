# ---- Stage 1: build the WASM package from the Rust crate ----
FROM rust:1-bookworm AS wasm
RUN cargo install wasm-pack
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN wasm-pack build crates/praxsmth-wasm --target web --out-dir pkg --release

# ---- Stage 2: build the SvelteKit static site ----
FROM node:22-bookworm AS web
WORKDIR /app
COPY package.json package-lock.json ./
# bring in the wasm pkg first so the `file:` dependency resolves
COPY --from=wasm /app/crates/praxsmth-wasm/pkg ./crates/praxsmth-wasm/pkg
RUN npm ci
COPY . .
COPY --from=wasm /app/crates/praxsmth-wasm/pkg ./crates/praxsmth-wasm/pkg
RUN npm run build

# ---- Stage 3: serve ----
FROM nginx:alpine
COPY docker/nginx.conf /etc/nginx/conf.d/default.conf
COPY --from=web /app/build /usr/share/nginx/html
EXPOSE 80
