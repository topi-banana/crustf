//! Covers `athrow` and the exception_table of a `Code` attribute.

use crustf::spec::attribute::{Attribute, CodeAttribute, ExceptionHandler};
use crustf::spec::instruction::Instruction;
use crustf::spec::{ClassAccess, ClassFile, ConstantPool, MethodAccess, JAVA_5};
use crustf_jvm_integration_tests as harness;

#[test]
fn try_catch_runtime_exception() {
    // `main`:
    //   try { 1 / 0; println("miss"); }
    //   catch (ArithmeticException) { println("caught"); }
    //
    // The spec crate is used directly because the public builder does not
    // yet expose `exception_table`; the test doubles as coverage for the
    // encoder's handler-table path.
    let mut pool = ConstantPool::new();
    let this_class = pool.intern_class("TryCatch").unwrap();
    let super_class = pool.intern_class("java/lang/Object").unwrap();
    let obj_init = pool
        .intern_methodref("java/lang/Object", "<init>", "()V")
        .unwrap();
    let system_out = pool
        .intern_fieldref("java/lang/System", "out", "Ljava/io/PrintStream;")
        .unwrap();
    let println_str = pool
        .intern_methodref("java/io/PrintStream", "println", "(Ljava/lang/String;)V")
        .unwrap();
    let caught = pool.intern_string("caught").unwrap();
    let miss = pool.intern_string("miss").unwrap();
    let arithmetic = pool.intern_class("java/lang/ArithmeticException").unwrap();
    let init_name = pool.intern_utf8("<init>").unwrap();
    let init_desc = pool.intern_utf8("()V").unwrap();
    let main_name = pool.intern_utf8("main").unwrap();
    let main_desc = pool.intern_utf8("([Ljava/lang/String;)V").unwrap();
    pool.intern_utf8("Code").unwrap();

    let init_code = {
        let mut code = Vec::<u8>::new();
        Instruction::Aload0.encode(&mut code).unwrap();
        Instruction::Invokespecial(obj_init)
            .encode(&mut code)
            .unwrap();
        Instruction::Return.encode(&mut code).unwrap();
        code
    };

    // Build the main body twice so we can resolve `goto` to an absolute
    // offset without hand-computing it: encode the try block, the goto
    // (with a placeholder), then the handler, then patch the goto.
    let mut code = Vec::<u8>::new();
    let try_start = code.len() as u16;
    Instruction::Iconst1.encode(&mut code).unwrap();
    Instruction::Iconst0.encode(&mut code).unwrap();
    Instruction::Idiv.encode(&mut code).unwrap();
    Instruction::Pop.encode(&mut code).unwrap();
    Instruction::Getstatic(system_out)
        .encode(&mut code)
        .unwrap();
    Instruction::LdcW(miss).encode(&mut code).unwrap();
    Instruction::Invokevirtual(println_str)
        .encode(&mut code)
        .unwrap();
    let try_end = code.len() as u16;
    let goto_pos = code.len();
    Instruction::Goto(0).encode(&mut code).unwrap();

    let handler_start = code.len() as u16;
    Instruction::Astore1.encode(&mut code).unwrap();
    Instruction::Getstatic(system_out)
        .encode(&mut code)
        .unwrap();
    Instruction::LdcW(caught).encode(&mut code).unwrap();
    Instruction::Invokevirtual(println_str)
        .encode(&mut code)
        .unwrap();
    let after = code.len();
    Instruction::Return.encode(&mut code).unwrap();

    let rel = (after as i32 - goto_pos as i32) as i16;
    code[goto_pos + 1..goto_pos + 3].copy_from_slice(&rel.to_be_bytes());

    let init_method = crustf::spec::Method {
        access_flags: MethodAccess::PUBLIC,
        name_index: init_name,
        descriptor_index: init_desc,
        attributes: vec![Attribute::Code(CodeAttribute {
            max_stack: 1,
            max_locals: 1,
            code: init_code,
            exception_table: vec![],
            attributes: vec![],
        })],
    };
    let main_method = crustf::spec::Method {
        access_flags: MethodAccess::PUBLIC | MethodAccess::STATIC,
        name_index: main_name,
        descriptor_index: main_desc,
        attributes: vec![Attribute::Code(CodeAttribute {
            max_stack: 2,
            max_locals: 2,
            code,
            exception_table: vec![ExceptionHandler {
                start_pc: try_start,
                end_pc: try_end,
                handler_pc: handler_start,
                catch_type: arithmetic,
            }],
            attributes: vec![],
        })],
    };
    let cf = ClassFile {
        version: JAVA_5,
        constant_pool: pool,
        access_flags: ClassAccess::PUBLIC | ClassAccess::SUPER,
        this_class,
        super_class,
        interfaces: vec![],
        fields: vec![],
        methods: vec![init_method, main_method],
        attributes: vec![],
    };

    let bytes = crustf::spec::encode(&cf).unwrap();
    let Some(stdout) = harness::run("TryCatch", &bytes, &[]) else {
        return;
    };
    assert_eq!(stdout.trim(), "caught");
}
