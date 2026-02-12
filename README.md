
<!-- markdownlint-disable MD033 -->

<!-- markdownlint-disable MD041 -->

<img align="right" width="250" src="assets/logo/tilt_svg.svg" alt="Super Lune logo" />

<h1 align="center">Super Lune</h1>

<div align="center">
	<div>
		<a href="https://github.com/thekingofspace/super_lune/releases">
			<img src="https://img.shields.io/github/v/release/thekingofspace/super_lune?label=Release" alt="Latest Super Lune release" />
		</a>
		<a href="https://github.com/thekingofspace/super_lune/actions">
			<img src="https://shields.io/endpoint?url=https://badges.readysetplay.io/workflow/thekingofspace/super_lune/ci.yaml" alt="CI status" />
		</a>
		<a href="https://github.com/thekingofspace/super_lune/blob/main/LICENSE.txt">
			<img src="https://img.shields.io/github/license/thekingofspace/super_lune.svg?label=License&color=informational" alt="License" />
		</a>
	</div>
</div>

<br/>

A evolution of [Lune](https://github.com/lune-org/lune), the standalone [Luau](https://luau-lang.org) runtime.

Super Lune builds on Lune’s asynchronous, Rust-powered foundation — adding enhanced development workflows, build flexibility, and deeper integration capabilities. with stuff like mongo, UDP, TCP, multithreading and more.

---

## Installation

Super Lune can be used in two ways:

### 📦 Option 1 — Use Prebuilt Releases

Download the latest binary from:

```
https://github.com/thekingofspace/super_lune/releases
```

Extract and run directly:

```bash
[super lune exe] run script.luau
```

---

### Option 2 — Build from Source

Clone the repository:

```bash
git clone https://github.com/thekingofspace/super_lune.git
cd super_lune
```

Build in release mode:

```bash
cargo build --release
```

Binary will be located at:

```
target/debug
```
