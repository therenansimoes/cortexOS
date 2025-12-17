# Deploy CortexOS no iPhone via USB-C

## Pré-requisitos
- Xcode instalado
- Apple Developer account (free ou pago)
- iPhone com iOS 14+

## Passo 1: Setup inicial (uma vez)
```bash
make setup
```

## Passo 2: Build Rust → iOS
```bash
make build
```

Gera: `target/aarch64-apple-ios/release/libcortex_ios_ffi.a`

## Passo 3: Deploy
```bash
# Plugar iPhone via USB-C
make deploy
```

Isso abre o Xcode project. Então:

1. **No Xcode, selecione seu iPhone** como target (topo, perto do ▶️)
2. **Clique o botão ▶️ (Run)**
3. **Authorize no iPhone** (Trust → Trust Developer)
4. App instala e roda no device

## Troubleshooting

**"Cannot find libcortex_ios_ffi.a"**
```bash
make build
```

**"Team ID not set"**
- Xcode → Project → Signing & Capabilities
- Team: selecione sua Apple ID

**Build fails com linker error**
```bash
cargo clean
make build
```

## Rebuild after code changes
```bash
make build && make deploy
```
