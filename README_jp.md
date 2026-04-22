# crustf

Rust だけで JVM クラスファイルを直接吐き出せる、アセンブラ風 fluent API 付きのクレート群です。

`crustf` を使うと `javac` を経由せずに `.class` を生成できます。`alloc`
だけに依存した `no_std` 対応で、`wasm32-unknown-unknown` と
`wasm32-wasip1` ターゲットでも問題なく動くので、Wasm ツールチェーンや
`alloc` だけが使える組み込み用途でもそのまま使えます。

```rust
use crustf::{AccessFlags, ClassFileBuilder, MethodBuilder};

let bytes = ClassFileBuilder::new("TestHello")
    .method(
        MethodBuilder::new("<init>", "()V")
            .access_flags(AccessFlags::PUBLIC)
            .code(|c| {
                c.aload(0)
                 .invokespecial("java/lang/Object", "<init>", "()V")
                 .return_void();
            }),
    )
    .method(
        MethodBuilder::new("main", "([Ljava/lang/String;)V")
            .access_flags(AccessFlags::PUBLIC | AccessFlags::STATIC)
            .code(|code| {
                code.getstatic("java/lang/System", "out", "Ljava/io/PrintStream;")
                    .ldc_string("Hello from assemb!")
                    .invokevirtual("java/io/PrintStream", "println", "(Ljava/lang/String;)V")
                    .return_void();
            }),
    )
    .build()?;

std::fs::write("TestHello.class", &bytes)?;
# Ok::<_, crustf::Error>(())
```

```
$ java TestHello
Hello from assemb!
```

## ワークスペース構成

```
crustf/
├── crates/
│   ├── crustf-spec/   データモデルとバイナリエンコーダ (JVMS §4)
│   ├── crustf-asm/    fluent ビルダー層 (ラベル・プール解決)
│   └── crustf/        クレート利用者向けの再エクスポート
└── tests/jvm-integration/   生成バイト列を `java` で実際に動かす E2E
```

各クレートは `cargo doc --open` でドキュメントを確認できます。依存方向は
`crustf-spec → crustf-asm → crustf` で固定なので、ビルダーが不要なら下位
クレートだけに依存することもできます。

### `crustf-spec`

* `ClassFile`、`ConstantPool`、属性、`field_info`、`method_info`、JVM 命令
  を JVMS (Java SE 25) に沿って網羅的にモデル化。
* JVMS 準拠の順序でバイト列に書き出すエンコーダ。
* ASCII 高速パスとサロゲート対応つき modified UTF-8 コーデック。
* 200 種以上の opcode (予約された `breakpoint`、`impdep1`、`impdep2` を含
  む) と全 `wide` 変形命令を enum バリアントで表現。
* `std` feature を無効化すれば `#![no_std]`。

### `crustf-asm`

* `ClassFileBuilder` / `FieldBuilder` / `MethodBuilder` / `CodeBuilder`。
* 統合 [`AccessFlags`][af] がクラス/フィールド/メソッド/パラメータの
  4 コンテキストを束ね、spec 側に渡すタイミングで余分なビットを切り落と
  します。
* ラベル、`tableswitch` / `lookupswitch`、短距離・長距離ブランチは
  `.build()` 時に自動解決。
* 自動ナロー: `iload(0)` → `iload_0`、`iload(5)` → `iload 5`、
  `iload(500)` → `wide iload 500`。store や `iinc` も同様。
* `ldc` / `ldc_w` / `ldc2_w` の選択も解決済みプールインデックスから自動
  判断。
* `max_locals` は引数スロット数と使用中ローカルから推定。

[af]: crates/crustf-asm/src/access.rs

## クラスファイルバージョンの既定値

`ClassFileBuilder::new(...)` はメジャーバージョン `49` (Java SE 5) を既
定値として採用しています。メジャー番号 ≤ 49 のクラスファイルは旧式の
型推論ベリファイアで検証されるため、`StackMapTable` 属性なしでもバック
ブランチを含む単純なビルドが合格します。`invokedynamic`、ダイナミック定
数、sealed クラスなど現代機能が必要なときは `.version(JAVA_17)` のように
明示的に上げてください。その場合は正しい `StackMapTable` を自前で用意す
る必要があります。

## テスト

`tests/jvm-integration` クレートは `crustf` で生成したクラスファイルを
実際の `java` バイナリで走らせ、標準出力を期待値と突き合わせます。
`tests/jvm-integration/tests/` の各ファイルで命令カテゴリごとに異なる面
をカバーしています:

| ファイル | 対象 |
| --- | --- |
| `hello_world.rs` | `getstatic`、`ldc_string`、`invokevirtual` |
| `arithmetic.rs` | `i/l/f/d add/sub/mul/div/rem`、`ishl`、`iand`、`ior`、`ixor` |
| `control_flow.rs` | `if_icmp*`、`goto`、`iinc`、`tableswitch`、`lookupswitch` |
| `arrays.rs` | `newarray`、`anewarray`、`iaload`/`iastore`、`arraylength` |
| `references.rs` | `new`、`invokespecial`、`getfield`/`putfield`、`checkcast`、`instanceof`、`invokeinterface` |
| `constants.rs` | 各種 `iconst_*`、`lconst_*`、`bipush`、`sipush`、`ldc`、`ldc_w`、`ldc2_w` |
| `conversions.rs` | `i2l`、`i2f`、`i2d`、`l2i`、`l2d`、`d2i`、`i2b`、`i2c`、`i2s` |
| `comparisons.rs` | `lcmp`、`ifnull`、`ifnonnull` |
| `stack_ops.rs` | `dup`、`swap`、`pop` |
| `exceptions.rs` | `athrow`、`exception_table` (try/catch) |

全テストを実行:

```bash
cargo test --workspace
```

`java` が `PATH` になければテストは静かにスキップします。

## 継続的インテグレーション

`.github/workflows/ci.yml` で以下を実行します:

* `cargo fmt --all --check`
* `cargo clippy --workspace --all-targets --all-features -- -D warnings`
* `cargo build --workspace --all-targets --all-features`
* `cargo test  --workspace --all-features` (Temurin 21 を `PATH` に追加)
* `taplo format --check --diff` (Cargo.toml の整形チェック)
* `cargo machete` (未使用依存検出)
* `crustf-spec` と `crustf-asm` の `wasm32-unknown-unknown` / `wasm32-wasip1` クロスビルド

## サンプル実行

```bash
cargo run --example hello -p crustf-asm -- /tmp
cd /tmp && java HelloCrustf
# Hello from crustf!
```

## ライセンス

以下のいずれかを選択できます:

* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
* MIT license ([LICENSE-MIT](LICENSE-MIT))
