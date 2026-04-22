//! Fabric + Mixin Minecraft mod example.
//!
//! Builds a `HelloModMixin` class that uses SpongePowered Mixin to inject a
//! `println` at the head of `MinecraftServer#runServer`, emits the
//! `fabric.mod.json` and `hello-mod.mixins.json` descriptors Fabric Loader
//! needs, and packages everything into a JAR that can be dropped into
//! `<.minecraft>/mods/`.
//!
//! ```text
//! cargo run -p crustf-examples --bin minecraft_mod_test
//! cargo run -p crustf-examples --bin minecraft_mod_test -- /tmp/myout
//! ```
//!
//! The annotations on the class and method are the full Mixin shape —
//! `@Mixin(value = { MinecraftServer.class })` and `@Inject(method =
//! "runServer", at = @At("HEAD"))` — produced by [`crustf::Annotation`]
//! and [`crustf::ElementValue`] without reaching into the spec layer.

use std::collections::BTreeMap;
use std::env;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crustf::{AccessFlags, Annotation, ClassFileBuilder, ElementValue, MethodBuilder, Version};
use serde::Serialize;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

const CLASS_INTERNAL: &str = "com/example/mixin/HelloModMixin";
const MIXIN_PACKAGE: &str = "com.example.mixin";
const MOD_ID: &str = "hello-mod";

/// Subset of the Fabric Loader descriptor schema this example populates.
/// `BTreeMap` keeps the emitted JSON key order stable across runs — useful
/// when consumers diff the generated `fabric.mod.json`.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FabricMod {
    schema_version: u32,
    id: String,
    version: String,
    name: String,
    description: String,
    environment: String,
    license: String,
    mixins: Vec<String>,
    depends: BTreeMap<String, String>,
    authors: Vec<String>,
}

#[derive(Serialize)]
struct MixinConfig {
    required: bool,
    package: String,
    #[serde(rename = "compatibilityLevel")]
    compatibility_level: String,
    mixins: Vec<String>,
    injectors: MixinInjectors,
}

#[derive(Serialize)]
struct MixinInjectors {
    #[serde(rename = "defaultRequire")]
    default_require: u32,
}

fn main() -> ExitCode {
    let out_dir = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(env::temp_dir);
    match run(&out_dir) {
        Ok(jar) => {
            println!("wrote {}", jar.display());
            println!("drop it in <minecraft>/mods/ next to a Fabric Loader install");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(out_dir: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    fs::create_dir_all(out_dir)?;

    let class_bytes = build_mixin_class()?;
    let fabric_json = serde_json::to_string_pretty(&fabric_mod_descriptor())?;
    let mixin_json = serde_json::to_string_pretty(&mixin_config())?;

    let jar_path = out_dir.join(format!("{MOD_ID}.jar"));
    // `BufWriter` amortises the many small header writes the ZIP writer
    // performs per entry; without it each header is a separate syscall.
    let mut jar = ZipWriter::new(BufWriter::new(File::create(&jar_path)?));
    let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    jar.start_file("META-INF/MANIFEST.MF", opts)?;
    jar.write_all(b"Manifest-Version: 1.0\n")?;

    jar.start_file("fabric.mod.json", opts)?;
    jar.write_all(fabric_json.as_bytes())?;

    jar.start_file(format!("{MOD_ID}.mixins.json"), opts)?;
    jar.write_all(mixin_json.as_bytes())?;

    jar.start_file(format!("{CLASS_INTERNAL}.class"), opts)?;
    jar.write_all(&class_bytes)?;

    jar.finish()?;

    println!("--- fabric.mod.json ---");
    println!("{fabric_json}");
    println!();
    println!("--- {MOD_ID}.mixins.json ---");
    println!("{mixin_json}");
    println!();
    Ok(jar_path)
}

fn build_mixin_class() -> crustf::Result<Vec<u8>> {
    ClassFileBuilder::new(CLASS_INTERNAL)
        // Java 8 bytecode: Mixin requires at least 52.0, and the only
        // methods we emit are branchless so no StackMapTable is needed.
        .version(Version::new(52, 0))
        .annotation(
            Annotation::invisible("Lorg/spongepowered/asm/mixin/Mixin;").element(
                "value",
                ElementValue::Array(vec![ElementValue::Class(
                    "Lnet/minecraft/server/MinecraftServer;".into(),
                )]),
            ),
        )
        .method(
            MethodBuilder::new("<init>", "()V")
                .access_flags(AccessFlags::PUBLIC)
                .code(|code| {
                    code.aload(0)
                        .invokespecial("java/lang/Object", "<init>", "()V")
                        .return_void();
                }),
        )
        .method(
            MethodBuilder::new(
                "onRun",
                "(Lorg/spongepowered/asm/mixin/injection/callback/CallbackInfo;)V",
            )
            .access_flags(AccessFlags::PRIVATE)
            .annotation(
                Annotation::visible("Lorg/spongepowered/asm/mixin/injection/Inject;")
                    .element("method", ElementValue::String("runServer".into()))
                    .element(
                        "at",
                        ElementValue::Array(vec![ElementValue::from(
                            Annotation::visible("Lorg/spongepowered/asm/mixin/injection/At;")
                                .element("value", ElementValue::String("HEAD".into())),
                        )]),
                    ),
            )
            .code(|code| {
                code.getstatic("java/lang/System", "out", "Ljava/io/PrintStream;")
                    .ldc_string("HelloModMixin.onRun called!")
                    .invokevirtual("java/io/PrintStream", "println", "(Ljava/lang/String;)V")
                    .return_void();
            }),
        )
        .build()
}

fn fabric_mod_descriptor() -> FabricMod {
    FabricMod {
        schema_version: 1,
        id: MOD_ID.into(),
        version: "1.0.0".into(),
        name: "Hello Mod".into(),
        description: "A hello-world Fabric mod built with crustf".into(),
        environment: "*".into(),
        license: "MIT".into(),
        mixins: vec![format!("{MOD_ID}.mixins.json")],
        depends: BTreeMap::from([
            ("fabricloader".into(), ">=0.15.0".into()),
            ("minecraft".into(), ">=1.20".into()),
        ]),
        authors: vec!["crustf contributors".into()],
    }
}

fn mixin_config() -> MixinConfig {
    MixinConfig {
        required: true,
        package: MIXIN_PACKAGE.into(),
        compatibility_level: "JAVA_8".into(),
        mixins: vec!["HelloModMixin".into()],
        injectors: MixinInjectors { default_require: 1 },
    }
}
