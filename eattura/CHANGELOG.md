# Changelog

## [0.1.1](https://github.com/mindmain/eattura/compare/v0.1.0...v0.1.1) (2026-11-27)


### Bug Fixes

* **pec:** harden receipt sync with inbox adapters and transport status ([9f89f94](https://github.com/mindmain/eattura/commit/9f89f942013ce3bcf84d5ce309853ef3fc6bef15))

## 0.1.0 (2026-11-27)


### Features

* **app:** add local MCP tools and harden invoicing ([fbe6202](https://github.com/mindmain/eattura/commit/fbe62027ef50d40364efd673191704da8f6a200f))
* **app:** support MCP draft editing and recover PEC attempts ([a849434](https://github.com/mindmain/eattura/commit/a849434848c8dea358d4093b6eab5d9f6b009b8f))
* **clients:** show SdI codes in client views ([f5bd532](https://github.com/mindmain/eattura/commit/f5bd532bd6c102ed0d0468c5c0a0c8abd718ee3a))
* **drive:** import invoices from a connected folder ([18d8c29](https://github.com/mindmain/eattura/commit/18d8c2969dcaceb8eed5c22e3bac26545c1cae7b))
* **drive:** sync single structured backup archive ([5e240f6](https://github.com/mindmain/eattura/commit/5e240f6c2096de7b3c1b701dd4d29d103370f2d6))
* **invoices:** redesign draft editor and fix IPC payload ([2dbed85](https://github.com/mindmain/eattura/commit/2dbed85a482574849dc1ec4111cd0c584f0f0468))
* **invoices:** support service periods and advanced draft lines ([40f27c2](https://github.com/mindmain/eattura/commit/40f27c220f543f7dab6d7f9e240dcd60b987c278))
* **invoices:** surface delivered and PA refusal states ([bf7ab43](https://github.com/mindmain/eattura/commit/bf7ab434898ea645bcc3eaf6fb169ff5779cc20b))
* **invoices:** use SdI filenames for XML and PDF downloads ([69b35c4](https://github.com/mindmain/eattura/commit/69b35c4044a0744834d6b6c171f54d70709a1287))
* **mcp:** connect local assistants over stdio ([d1e4d50](https://github.com/mindmain/eattura/commit/d1e4d503c0c800eef06a902d5484b10711583453))
* **mcp:** patch draft descriptions and service periods ([5b07995](https://github.com/mindmain/eattura/commit/5b0799508c459d7297854f06f807182fcfaf5ad1))
* **sdi:** handle rejection receipts and explain MCP errors ([d57b150](https://github.com/mindmain/eattura/commit/d57b1508c270e1ea5c53eea546ff0d1c4dc504ef))
* **sdi:** separate delivered status and harden PEC ([e0e7679](https://github.com/mindmain/eattura/commit/e0e7679aa04587a895dcedb2ee29ec3c999cdb2e))
* **updater:** add signed GitHub release updates ([d397b81](https://github.com/mindmain/eattura/commit/d397b81f7917c4f393c0c13f4aaadc1a86ec69e6))


### Bug Fixes

* **dashboard:** chart sent sales and delivered revenue ([04ceac7](https://github.com/mindmain/eattura/commit/04ceac725089a57885d8571ed5d09b9dfb649535))
* **dashboard:** preserve imported sales in revenue ([b55593e](https://github.com/mindmain/eattura/commit/b55593eb9ddb96953c4d6d6bf20bf98e4c315192))
* **invoices:** clarify SdI status and align receipt actions ([dd5b9fc](https://github.com/mindmain/eattura/commit/dd5b9fc2659747194ab86a9e168e1b3d775b2347))
* **invoices:** compact columns and separate pending revenue ([7f09e36](https://github.com/mindmain/eattura/commit/7f09e36240f23f30f7fb06d108537d8d04e961e4))
* **pec:** isolate SDI outcomes by transmission attempt ([0ea76ad](https://github.com/mindmain/eattura/commit/0ea76ad0575c06ce325ef9a0177cff45d0654c51))
* **sdi:** enforce line description format across invoice flows ([1d7fae1](https://github.com/mindmain/eattura/commit/1d7fae14d3c49289677906265ca6fbcfee2e5fa1))
* **sdi:** expand local invoice validation ([741b0a9](https://github.com/mindmain/eattura/commit/741b0a96108f55e56ba35293ea78379e9b796a75))
* **sdi:** use catalog fiscal codes for transmission ([6efc1cb](https://github.com/mindmain/eattura/commit/6efc1cbcc921084aa2cb6604da0a8e0851ee7b9d))
* **sdi:** validate Italian tax identifiers ([3f3b5ee](https://github.com/mindmain/eattura/commit/3f3b5ee6018c85a583f0288e14c0e1769576e7bd))
