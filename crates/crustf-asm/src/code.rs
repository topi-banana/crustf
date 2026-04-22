//! Method body builder producing a `CodeAttribute` after symbol resolution.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crustf_spec::attribute::{Attribute, CodeAttribute, LineNumberEntry};
use crustf_spec::constant::{Constant, ConstantPool, ReferenceKind};
use crustf_spec::error::{Error, Result};
use crustf_spec::instruction::{ArrayType, Instruction, WideInstruction};

use crate::label::{Label, LabelTable};

/// Opcodes for fluent field access.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FieldOp {
    GetStatic,
    PutStatic,
    GetField,
    PutField,
}

/// Opcodes for fluent method invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InvokeOp {
    Virtual,
    Special,
    Static,
    Interface,
}

/// Opcodes that reference a single class (`new`, `anewarray`, `checkcast`, `instanceof`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ClassRefOp {
    New,
    Anewarray,
    Checkcast,
    Instanceof,
}

/// Conditional and unconditional short branches (16-bit signed offset).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BranchKind {
    Ifeq,
    Ifne,
    Iflt,
    Ifge,
    Ifgt,
    Ifle,
    IfIcmpeq,
    IfIcmpne,
    IfIcmplt,
    IfIcmpge,
    IfIcmpgt,
    IfIcmple,
    IfAcmpeq,
    IfAcmpne,
    Goto,
    Jsr,
    Ifnull,
    Ifnonnull,
}

impl BranchKind {
    /// Route short-branch emission through `Instruction::encode` so the
    /// opcode table stays single-sourced in `crustf_spec`.
    const fn to_instruction(self, offset: i16) -> Instruction {
        match self {
            Self::Ifeq => Instruction::Ifeq(offset),
            Self::Ifne => Instruction::Ifne(offset),
            Self::Iflt => Instruction::Iflt(offset),
            Self::Ifge => Instruction::Ifge(offset),
            Self::Ifgt => Instruction::Ifgt(offset),
            Self::Ifle => Instruction::Ifle(offset),
            Self::IfIcmpeq => Instruction::IfIcmpeq(offset),
            Self::IfIcmpne => Instruction::IfIcmpne(offset),
            Self::IfIcmplt => Instruction::IfIcmplt(offset),
            Self::IfIcmpge => Instruction::IfIcmpge(offset),
            Self::IfIcmpgt => Instruction::IfIcmpgt(offset),
            Self::IfIcmple => Instruction::IfIcmple(offset),
            Self::IfAcmpeq => Instruction::IfAcmpeq(offset),
            Self::IfAcmpne => Instruction::IfAcmpne(offset),
            Self::Goto => Instruction::Goto(offset),
            Self::Jsr => Instruction::Jsr(offset),
            Self::Ifnull => Instruction::Ifnull(offset),
            Self::Ifnonnull => Instruction::Ifnonnull(offset),
        }
    }
}

/// An `ldc`/`ldc_w`/`ldc2_w` payload whose final opcode is determined by the
/// interned pool index during resolution.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum LdcValue {
    Int(i32),
    Long(i64),
    Float(u32),
    Double(u64),
    String(String),
    Class(String),
    MethodType(String),
    MethodHandle {
        kind: ReferenceKind,
        reference_index: u16,
    },
}

/// One entry in the builder's working list.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Op {
    Raw(Instruction),
    Ldc(LdcValue),
    Field {
        op: FieldOp,
        class: String,
        name: String,
        desc: String,
    },
    Invoke {
        op: InvokeOp,
        class: String,
        name: String,
        desc: String,
    },
    InvokeDynamic {
        bootstrap_method_attr_index: u16,
        name: String,
        desc: String,
    },
    ClassRef {
        op: ClassRefOp,
        class: String,
    },
    Multianewarray {
        class: String,
        dimensions: u8,
    },
    Placed(Label),
    Branch {
        kind: BranchKind,
        target: Label,
    },
    GotoW(Label),
    JsrW(Label),
    Tableswitch {
        default: Label,
        low: i32,
        cases: Vec<Label>,
    },
    Lookupswitch {
        default: Label,
        pairs: Vec<(i32, Label)>,
    },
    LineNumber(u16),
}

/// Fluent builder for the body of a method.
#[derive(Debug, Default)]
pub struct CodeBuilder {
    pub(crate) ops: Vec<Op>,
    pub(crate) labels: LabelTable,
    pub(crate) max_stack: Option<u16>,
    pub(crate) max_locals: Option<u16>,
}

macro_rules! raw_methods {
    ($($name:ident => $variant:ident,)*) => {
        $(
            #[doc = concat!("Emit `", stringify!($name), "` (JVMS §6.5.", stringify!($name), ").")]
            pub fn $name(&mut self) -> &mut Self {
                self.ops.push(Op::Raw(Instruction::$variant));
                self
            }
        )*
    };
}

impl CodeBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Request a new fresh label. Place it at the desired code position with
    /// [`CodeBuilder::place`].
    pub fn label(&mut self) -> Label {
        self.labels.new_label()
    }

    /// Attach a previously created label to the *next* instruction emitted.
    pub fn place(&mut self, label: Label) -> &mut Self {
        self.ops.push(Op::Placed(label));
        self
    }

    /// Force `max_stack`. Leaving this unset asks the resolver to use a
    /// conservative ceiling (the argument count of the widest instruction).
    pub fn max_stack(&mut self, v: u16) -> &mut Self {
        self.max_stack = Some(v);
        self
    }

    /// Force `max_locals`. Leaving this unset uses the widest local slot
    /// index mentioned in the emitted instructions (+1 for slot count).
    pub fn max_locals(&mut self, v: u16) -> &mut Self {
        self.max_locals = Some(v);
        self
    }

    /// Associate the next instruction with a source line (emitted as a
    /// `LineNumberTable` entry).
    pub fn line(&mut self, line: u16) -> &mut Self {
        self.ops.push(Op::LineNumber(line));
        self
    }

    /// Escape hatch for instructions that have no dedicated builder method.
    pub fn raw(&mut self, ins: Instruction) -> &mut Self {
        self.ops.push(Op::Raw(ins));
        self
    }

    raw_methods! {
        nop => Nop,
        aconst_null => AconstNull,
        iconst_m1 => IconstM1,
        iconst_0 => Iconst0,
        iconst_1 => Iconst1,
        iconst_2 => Iconst2,
        iconst_3 => Iconst3,
        iconst_4 => Iconst4,
        iconst_5 => Iconst5,
        lconst_0 => Lconst0,
        lconst_1 => Lconst1,
        fconst_0 => Fconst0,
        fconst_1 => Fconst1,
        fconst_2 => Fconst2,
        dconst_0 => Dconst0,
        dconst_1 => Dconst1,
        iaload => Iaload,
        laload => Laload,
        faload => Faload,
        daload => Daload,
        aaload => Aaload,
        baload => Baload,
        caload => Caload,
        saload => Saload,
        iastore => Iastore,
        lastore => Lastore,
        fastore => Fastore,
        dastore => Dastore,
        aastore => Aastore,
        bastore => Bastore,
        castore => Castore,
        sastore => Sastore,
        pop => Pop,
        pop2 => Pop2,
        dup => Dup,
        dup_x1 => DupX1,
        dup_x2 => DupX2,
        dup2 => Dup2,
        dup2_x1 => Dup2X1,
        dup2_x2 => Dup2X2,
        swap => Swap,
        iadd => Iadd,
        ladd => Ladd,
        fadd => Fadd,
        dadd => Dadd,
        isub => Isub,
        lsub => Lsub,
        fsub => Fsub,
        dsub => Dsub,
        imul => Imul,
        lmul => Lmul,
        fmul => Fmul,
        dmul => Dmul,
        idiv => Idiv,
        ldiv => Ldiv,
        fdiv => Fdiv,
        ddiv => Ddiv,
        irem => Irem,
        lrem => Lrem,
        frem => Frem,
        drem => Drem,
        ineg => Ineg,
        lneg => Lneg,
        fneg => Fneg,
        dneg => Dneg,
        ishl => Ishl,
        lshl => Lshl,
        ishr => Ishr,
        lshr => Lshr,
        iushr => Iushr,
        lushr => Lushr,
        iand => Iand,
        land => Land,
        ior => Ior,
        lor => Lor,
        ixor => Ixor,
        lxor => Lxor,
        i2l => I2l,
        i2f => I2f,
        i2d => I2d,
        l2i => L2i,
        l2f => L2f,
        l2d => L2d,
        f2i => F2i,
        f2l => F2l,
        f2d => F2d,
        d2i => D2i,
        d2l => D2l,
        d2f => D2f,
        i2b => I2b,
        i2c => I2c,
        i2s => I2s,
        lcmp => Lcmp,
        fcmpl => Fcmpl,
        fcmpg => Fcmpg,
        dcmpl => Dcmpl,
        dcmpg => Dcmpg,
        ireturn => Ireturn,
        lreturn => Lreturn,
        freturn => Freturn,
        dreturn => Dreturn,
        areturn => Areturn,
        arraylength => Arraylength,
        athrow => Athrow,
        monitorenter => Monitorenter,
        monitorexit => Monitorexit,
        breakpoint => Breakpoint,
    }

    pub fn bipush(&mut self, v: i8) -> &mut Self {
        self.ops.push(Op::Raw(Instruction::Bipush(v)));
        self
    }

    pub fn sipush(&mut self, v: i16) -> &mut Self {
        self.ops.push(Op::Raw(Instruction::Sipush(v)));
        self
    }

    /// `iinc index, delta` – automatically widens to `wide iinc` when the
    /// operands exceed the 8-bit forms.
    pub fn iinc(&mut self, index: u16, delta: i16) -> &mut Self {
        let narrow = u8::try_from(index).ok().zip(i8::try_from(delta).ok());
        let ins = if let Some((i, d)) = narrow {
            Instruction::Iinc { index: i, delta: d }
        } else {
            Instruction::Wide(WideInstruction::Iinc { index, delta })
        };
        self.ops.push(Op::Raw(ins));
        self
    }

    /// `ret index` – widens as needed.
    pub fn ret(&mut self, index: u16) -> &mut Self {
        let ins = if let Ok(i) = u8::try_from(index) {
            Instruction::Ret(i)
        } else {
            Instruction::Wide(WideInstruction::Ret(index))
        };
        self.ops.push(Op::Raw(ins));
        self
    }

    pub fn newarray(&mut self, atype: ArrayType) -> &mut Self {
        self.ops.push(Op::Raw(Instruction::Newarray(atype)));
        self
    }

    pub fn return_void(&mut self) -> &mut Self {
        self.ops.push(Op::Raw(Instruction::Return));
        self
    }

    // Loads / stores with auto narrow/wide/short selection.
    pub fn iload(&mut self, index: u16) -> &mut Self {
        self.push_load(index, 0x1a, Instruction::Iload, WideInstruction::Iload)
    }
    pub fn lload(&mut self, index: u16) -> &mut Self {
        self.push_load(index, 0x1e, Instruction::Lload, WideInstruction::Lload)
    }
    pub fn fload(&mut self, index: u16) -> &mut Self {
        self.push_load(index, 0x22, Instruction::Fload, WideInstruction::Fload)
    }
    pub fn dload(&mut self, index: u16) -> &mut Self {
        self.push_load(index, 0x26, Instruction::Dload, WideInstruction::Dload)
    }
    pub fn aload(&mut self, index: u16) -> &mut Self {
        self.push_load(index, 0x2a, Instruction::Aload, WideInstruction::Aload)
    }
    pub fn istore(&mut self, index: u16) -> &mut Self {
        self.push_load(index, 0x3b, Instruction::Istore, WideInstruction::Istore)
    }
    pub fn lstore(&mut self, index: u16) -> &mut Self {
        self.push_load(index, 0x3f, Instruction::Lstore, WideInstruction::Lstore)
    }
    pub fn fstore(&mut self, index: u16) -> &mut Self {
        self.push_load(index, 0x43, Instruction::Fstore, WideInstruction::Fstore)
    }
    pub fn dstore(&mut self, index: u16) -> &mut Self {
        self.push_load(index, 0x47, Instruction::Dstore, WideInstruction::Dstore)
    }
    pub fn astore(&mut self, index: u16) -> &mut Self {
        self.push_load(index, 0x4b, Instruction::Astore, WideInstruction::Astore)
    }

    fn push_load(
        &mut self,
        index: u16,
        shortcut_base: u8,
        basic: fn(u8) -> Instruction,
        wide: fn(u16) -> WideInstruction,
    ) -> &mut Self {
        // shortcut_base is the opcode for `*_0`. `*_1..3` are consecutive.
        // All shortcut opcodes in JVMS come in blocks of four.
        let ins = if index < 4 {
            raw_from_opcode(shortcut_base + index as u8)
        } else if let Ok(i) = u8::try_from(index) {
            basic(i)
        } else {
            Instruction::Wide(wide(index))
        };
        self.ops.push(Op::Raw(ins));
        self
    }

    // `ldc` family – narrow or wide chosen during resolution.
    pub fn ldc_int(&mut self, v: i32) -> &mut Self {
        self.ops.push(Op::Ldc(LdcValue::Int(v)));
        self
    }
    pub fn ldc_long(&mut self, v: i64) -> &mut Self {
        self.ops.push(Op::Ldc(LdcValue::Long(v)));
        self
    }
    pub fn ldc_float(&mut self, v: f32) -> &mut Self {
        self.ops.push(Op::Ldc(LdcValue::Float(v.to_bits())));
        self
    }
    pub fn ldc_double(&mut self, v: f64) -> &mut Self {
        self.ops.push(Op::Ldc(LdcValue::Double(v.to_bits())));
        self
    }
    pub fn ldc_string(&mut self, s: impl Into<String>) -> &mut Self {
        self.ops.push(Op::Ldc(LdcValue::String(s.into())));
        self
    }
    pub fn ldc_class(&mut self, internal_name: impl Into<String>) -> &mut Self {
        self.ops
            .push(Op::Ldc(LdcValue::Class(internal_name.into())));
        self
    }
    pub fn ldc_method_type(&mut self, descriptor: impl Into<String>) -> &mut Self {
        self.ops
            .push(Op::Ldc(LdcValue::MethodType(descriptor.into())));
        self
    }
    pub fn ldc_method_handle(&mut self, kind: ReferenceKind, reference_index: u16) -> &mut Self {
        self.ops.push(Op::Ldc(LdcValue::MethodHandle {
            kind,
            reference_index,
        }));
        self
    }

    // Field access.
    pub fn getstatic(&mut self, class: &str, name: &str, desc: &str) -> &mut Self {
        self.push_field(FieldOp::GetStatic, class, name, desc)
    }
    pub fn putstatic(&mut self, class: &str, name: &str, desc: &str) -> &mut Self {
        self.push_field(FieldOp::PutStatic, class, name, desc)
    }
    pub fn getfield(&mut self, class: &str, name: &str, desc: &str) -> &mut Self {
        self.push_field(FieldOp::GetField, class, name, desc)
    }
    pub fn putfield(&mut self, class: &str, name: &str, desc: &str) -> &mut Self {
        self.push_field(FieldOp::PutField, class, name, desc)
    }
    fn push_field(&mut self, op: FieldOp, class: &str, name: &str, desc: &str) -> &mut Self {
        self.ops.push(Op::Field {
            op,
            class: class.to_string(),
            name: name.to_string(),
            desc: desc.to_string(),
        });
        self
    }

    // Method invocation.
    pub fn invokevirtual(&mut self, class: &str, name: &str, desc: &str) -> &mut Self {
        self.push_invoke(InvokeOp::Virtual, class, name, desc)
    }
    pub fn invokespecial(&mut self, class: &str, name: &str, desc: &str) -> &mut Self {
        self.push_invoke(InvokeOp::Special, class, name, desc)
    }
    pub fn invokestatic(&mut self, class: &str, name: &str, desc: &str) -> &mut Self {
        self.push_invoke(InvokeOp::Static, class, name, desc)
    }
    pub fn invokeinterface(&mut self, class: &str, name: &str, desc: &str) -> &mut Self {
        self.push_invoke(InvokeOp::Interface, class, name, desc)
    }
    fn push_invoke(&mut self, op: InvokeOp, class: &str, name: &str, desc: &str) -> &mut Self {
        self.ops.push(Op::Invoke {
            op,
            class: class.to_string(),
            name: name.to_string(),
            desc: desc.to_string(),
        });
        self
    }

    pub fn invokedynamic(
        &mut self,
        bootstrap_method_attr_index: u16,
        name: &str,
        desc: &str,
    ) -> &mut Self {
        self.ops.push(Op::InvokeDynamic {
            bootstrap_method_attr_index,
            name: name.to_string(),
            desc: desc.to_string(),
        });
        self
    }

    // Class references.
    pub fn new_class(&mut self, internal_name: &str) -> &mut Self {
        self.push_class(ClassRefOp::New, internal_name)
    }
    pub fn anewarray(&mut self, element_internal_name: &str) -> &mut Self {
        self.push_class(ClassRefOp::Anewarray, element_internal_name)
    }
    pub fn checkcast(&mut self, internal_name: &str) -> &mut Self {
        self.push_class(ClassRefOp::Checkcast, internal_name)
    }
    pub fn instanceof(&mut self, internal_name: &str) -> &mut Self {
        self.push_class(ClassRefOp::Instanceof, internal_name)
    }
    fn push_class(&mut self, op: ClassRefOp, internal_name: &str) -> &mut Self {
        self.ops.push(Op::ClassRef {
            op,
            class: internal_name.to_string(),
        });
        self
    }

    pub fn multianewarray(&mut self, internal_name: &str, dimensions: u8) -> &mut Self {
        self.ops.push(Op::Multianewarray {
            class: internal_name.to_string(),
            dimensions,
        });
        self
    }

    // Branches.
    pub fn ifeq(&mut self, target: Label) -> &mut Self {
        self.branch(BranchKind::Ifeq, target)
    }
    pub fn ifne(&mut self, target: Label) -> &mut Self {
        self.branch(BranchKind::Ifne, target)
    }
    pub fn iflt(&mut self, target: Label) -> &mut Self {
        self.branch(BranchKind::Iflt, target)
    }
    pub fn ifge(&mut self, target: Label) -> &mut Self {
        self.branch(BranchKind::Ifge, target)
    }
    pub fn ifgt(&mut self, target: Label) -> &mut Self {
        self.branch(BranchKind::Ifgt, target)
    }
    pub fn ifle(&mut self, target: Label) -> &mut Self {
        self.branch(BranchKind::Ifle, target)
    }
    pub fn if_icmpeq(&mut self, target: Label) -> &mut Self {
        self.branch(BranchKind::IfIcmpeq, target)
    }
    pub fn if_icmpne(&mut self, target: Label) -> &mut Self {
        self.branch(BranchKind::IfIcmpne, target)
    }
    pub fn if_icmplt(&mut self, target: Label) -> &mut Self {
        self.branch(BranchKind::IfIcmplt, target)
    }
    pub fn if_icmpge(&mut self, target: Label) -> &mut Self {
        self.branch(BranchKind::IfIcmpge, target)
    }
    pub fn if_icmpgt(&mut self, target: Label) -> &mut Self {
        self.branch(BranchKind::IfIcmpgt, target)
    }
    pub fn if_icmple(&mut self, target: Label) -> &mut Self {
        self.branch(BranchKind::IfIcmple, target)
    }
    pub fn if_acmpeq(&mut self, target: Label) -> &mut Self {
        self.branch(BranchKind::IfAcmpeq, target)
    }
    pub fn if_acmpne(&mut self, target: Label) -> &mut Self {
        self.branch(BranchKind::IfAcmpne, target)
    }
    pub fn goto(&mut self, target: Label) -> &mut Self {
        self.branch(BranchKind::Goto, target)
    }
    pub fn jsr(&mut self, target: Label) -> &mut Self {
        self.branch(BranchKind::Jsr, target)
    }
    pub fn ifnull(&mut self, target: Label) -> &mut Self {
        self.branch(BranchKind::Ifnull, target)
    }
    pub fn ifnonnull(&mut self, target: Label) -> &mut Self {
        self.branch(BranchKind::Ifnonnull, target)
    }

    fn branch(&mut self, kind: BranchKind, target: Label) -> &mut Self {
        self.ops.push(Op::Branch { kind, target });
        self
    }

    pub fn goto_w(&mut self, target: Label) -> &mut Self {
        self.ops.push(Op::GotoW(target));
        self
    }
    pub fn jsr_w(&mut self, target: Label) -> &mut Self {
        self.ops.push(Op::JsrW(target));
        self
    }

    pub fn tableswitch(&mut self, default: Label, low: i32, cases: Vec<Label>) -> &mut Self {
        self.ops.push(Op::Tableswitch {
            default,
            low,
            cases,
        });
        self
    }
    pub fn lookupswitch(&mut self, default: Label, pairs: Vec<(i32, Label)>) -> &mut Self {
        self.ops.push(Op::Lookupswitch { default, pairs });
        self
    }

    /// Resolve all pending references and produce a [`CodeAttribute`]. The
    /// `min_locals` floor typically comes from the enclosing method's
    /// parameter slot count (JVMS §4.7.3 requires `max_locals` to be large
    /// enough to cover every parameter, including an implicit `this`).
    pub fn finish(mut self, pool: &mut ConstantPool, min_locals: u16) -> Result<CodeAttribute> {
        // Phase 1 – compute each op's byte offset inside `code`.
        let (offsets, total_size) = compute_layout(&self.ops)?;
        // Phase 2 – place every label up front so forward branches resolve.
        for (op, offset) in self.ops.iter().zip(offsets.iter()) {
            if let Op::Placed(label) = op {
                self.labels.place(*label, *offset);
            }
        }
        // Phase 3 – emit bytes and accumulate debug info.
        let mut code = Vec::with_capacity(total_size);
        let mut line_numbers: Vec<LineNumberEntry> = Vec::new();
        let mut pending_line: Option<u16> = None;

        for (op, offset) in self.ops.iter().zip(offsets.iter()) {
            // Zero-size pseudo-ops bookkeep metadata before any byte emission.
            match op {
                Op::LineNumber(line) => {
                    pending_line = Some(*line);
                    continue;
                }
                Op::Placed(_) => continue,
                _ => {}
            }
            if let Some(line) = pending_line.take() {
                line_numbers.push(LineNumberEntry {
                    start_pc: *offset as u16,
                    line_number: line,
                });
            }
            match op {
                Op::Raw(ins) => ins.encode(&mut code)?,
                Op::Ldc(value) => resolve_ldc(value, pool)?.encode(&mut code)?,
                Op::Field {
                    op: fop,
                    class,
                    name,
                    desc,
                } => {
                    let idx = pool.intern_fieldref(class, name, desc)?;
                    let ins = match fop {
                        FieldOp::GetStatic => Instruction::Getstatic(idx),
                        FieldOp::PutStatic => Instruction::Putstatic(idx),
                        FieldOp::GetField => Instruction::Getfield(idx),
                        FieldOp::PutField => Instruction::Putfield(idx),
                    };
                    ins.encode(&mut code)?;
                }
                Op::Invoke {
                    op: iop,
                    class,
                    name,
                    desc,
                } => {
                    let ins = match iop {
                        InvokeOp::Virtual => {
                            Instruction::Invokevirtual(pool.intern_methodref(class, name, desc)?)
                        }
                        InvokeOp::Special => {
                            Instruction::Invokespecial(pool.intern_methodref(class, name, desc)?)
                        }
                        InvokeOp::Static => {
                            Instruction::Invokestatic(pool.intern_methodref(class, name, desc)?)
                        }
                        InvokeOp::Interface => Instruction::Invokeinterface {
                            index: pool.intern_interface_methodref(class, name, desc)?,
                            count: interface_arg_count(desc)?,
                        },
                    };
                    ins.encode(&mut code)?;
                }
                Op::InvokeDynamic {
                    bootstrap_method_attr_index,
                    name,
                    desc,
                } => {
                    let idx =
                        pool.intern_invoke_dynamic(*bootstrap_method_attr_index, name, desc)?;
                    Instruction::Invokedynamic(idx).encode(&mut code)?;
                }
                Op::ClassRef { op: cop, class } => {
                    let idx = pool.intern_class(class)?;
                    let ins = match cop {
                        ClassRefOp::New => Instruction::New(idx),
                        ClassRefOp::Anewarray => Instruction::Anewarray(idx),
                        ClassRefOp::Checkcast => Instruction::Checkcast(idx),
                        ClassRefOp::Instanceof => Instruction::Instanceof(idx),
                    };
                    ins.encode(&mut code)?;
                }
                Op::Multianewarray { class, dimensions } => {
                    let idx = pool.intern_class(class)?;
                    Instruction::Multianewarray {
                        index: idx,
                        dimensions: *dimensions,
                    }
                    .encode(&mut code)?;
                }
                Op::Branch { kind, target } => {
                    let rel16 = short_branch_offset(&self.labels, *target, *offset)?;
                    kind.to_instruction(rel16).encode(&mut code)?;
                }
                Op::GotoW(target) => {
                    Instruction::GotoW(long_branch_offset(&self.labels, *target, *offset)?)
                        .encode(&mut code)?;
                }
                Op::JsrW(target) => {
                    Instruction::JsrW(long_branch_offset(&self.labels, *target, *offset)?)
                        .encode(&mut code)?;
                }
                Op::Tableswitch {
                    default,
                    low,
                    cases,
                } => {
                    let default_rel = long_branch_offset(&self.labels, *default, *offset)?;
                    let mut offsets = Vec::with_capacity(cases.len());
                    for c in cases {
                        offsets.push(long_branch_offset(&self.labels, *c, *offset)?);
                    }
                    let high =
                        *low + i32::try_from(cases.len()).map_err(|_| Error::TooManyEntries {
                            what: "tableswitch cases",
                            count: cases.len(),
                        })? - 1;
                    Instruction::Tableswitch {
                        default: default_rel,
                        low: *low,
                        high,
                        offsets,
                    }
                    .encode(&mut code)?;
                }
                Op::Lookupswitch { default, pairs } => {
                    let default_rel = long_branch_offset(&self.labels, *default, *offset)?;
                    let mut resolved: Vec<(i32, i32)> = Vec::with_capacity(pairs.len());
                    for (m, l) in pairs {
                        resolved.push((*m, long_branch_offset(&self.labels, *l, *offset)?));
                    }
                    Instruction::Lookupswitch {
                        default: default_rel,
                        pairs: resolved,
                    }
                    .encode(&mut code)?;
                }
                Op::Placed(_) | Op::LineNumber(_) => unreachable!("handled above"),
            }
        }

        let inferred_locals = infer_max_locals(&self.ops).max(min_locals);
        let max_locals = self.max_locals.unwrap_or(inferred_locals);
        let max_stack = self.max_stack.unwrap_or(u16::MAX / 2);

        let mut attributes = Vec::new();
        if !line_numbers.is_empty() {
            crate::util::push_attr(
                pool,
                &mut attributes,
                Attribute::LineNumberTable(line_numbers),
            )?;
        }

        Ok(CodeAttribute {
            max_stack,
            max_locals,
            code,
            exception_table: Vec::new(),
            attributes,
        })
    }
}

fn branch_delta(labels: &LabelTable, target: Label, origin: u32) -> Result<i64> {
    let t = labels
        .resolve(target)
        .ok_or(Error::UnresolvedLabel(target.0))?;
    Ok(i64::from(t) - i64::from(origin))
}

fn short_branch_offset(labels: &LabelTable, target: Label, origin: u32) -> Result<i16> {
    let delta = branch_delta(labels, target, origin)?;
    i16::try_from(delta).map_err(|_| Error::BranchOutOfRange {
        offset: delta,
        width_bits: 16,
    })
}

fn long_branch_offset(labels: &LabelTable, target: Label, origin: u32) -> Result<i32> {
    let delta = branch_delta(labels, target, origin)?;
    i32::try_from(delta).map_err(|_| Error::BranchOutOfRange {
        offset: delta,
        width_bits: 32,
    })
}

fn interface_arg_count(desc: &str) -> Result<u8> {
    use crustf_spec::descriptor::MethodDescriptor;
    let md = MethodDescriptor::parse(desc)?;
    // `count` is 1 + param slots (the receiver plus the slot-weighted args).
    let total = 1 + md.param_slots();
    u8::try_from(total).map_err(|_| Error::TooManyEntries {
        what: "invokeinterface count",
        count: total as usize,
    })
}

fn raw_from_opcode(op: u8) -> Instruction {
    match op {
        0x1a => Instruction::Iload0,
        0x1b => Instruction::Iload1,
        0x1c => Instruction::Iload2,
        0x1d => Instruction::Iload3,
        0x1e => Instruction::Lload0,
        0x1f => Instruction::Lload1,
        0x20 => Instruction::Lload2,
        0x21 => Instruction::Lload3,
        0x22 => Instruction::Fload0,
        0x23 => Instruction::Fload1,
        0x24 => Instruction::Fload2,
        0x25 => Instruction::Fload3,
        0x26 => Instruction::Dload0,
        0x27 => Instruction::Dload1,
        0x28 => Instruction::Dload2,
        0x29 => Instruction::Dload3,
        0x2a => Instruction::Aload0,
        0x2b => Instruction::Aload1,
        0x2c => Instruction::Aload2,
        0x2d => Instruction::Aload3,
        0x3b => Instruction::Istore0,
        0x3c => Instruction::Istore1,
        0x3d => Instruction::Istore2,
        0x3e => Instruction::Istore3,
        0x3f => Instruction::Lstore0,
        0x40 => Instruction::Lstore1,
        0x41 => Instruction::Lstore2,
        0x42 => Instruction::Lstore3,
        0x43 => Instruction::Fstore0,
        0x44 => Instruction::Fstore1,
        0x45 => Instruction::Fstore2,
        0x46 => Instruction::Fstore3,
        0x47 => Instruction::Dstore0,
        0x48 => Instruction::Dstore1,
        0x49 => Instruction::Dstore2,
        0x4a => Instruction::Dstore3,
        0x4b => Instruction::Astore0,
        0x4c => Instruction::Astore1,
        0x4d => Instruction::Astore2,
        0x4e => Instruction::Astore3,
        _ => unreachable!("raw_from_opcode only called for load/store shortcut opcodes"),
    }
}

fn compute_layout(ops: &[Op]) -> Result<(Vec<u32>, usize)> {
    let mut offsets = Vec::with_capacity(ops.len());
    let mut pc: usize = 0;
    for op in ops {
        offsets.push(pc as u32);
        let size = match op {
            Op::Raw(ins) => ins.size(pc),
            Op::Ldc(_) => LDC_SIZE,
            Op::Field { .. }
            | Op::Invoke {
                op: InvokeOp::Virtual | InvokeOp::Special | InvokeOp::Static,
                ..
            }
            | Op::ClassRef { .. } => 3,
            Op::Invoke {
                op: InvokeOp::Interface,
                ..
            }
            | Op::InvokeDynamic { .. } => 5,
            Op::Multianewarray { .. } => 4,
            Op::Placed(_) | Op::LineNumber(_) => 0,
            Op::Branch { .. } => 3,
            Op::GotoW(_) | Op::JsrW(_) => 5,
            Op::Tableswitch { cases, .. } => {
                let pad = (4 - ((pc + 1) % 4)) % 4;
                1 + pad + 12 + 4 * cases.len()
            }
            Op::Lookupswitch { pairs, .. } => {
                let pad = (4 - ((pc + 1) % 4)) % 4;
                1 + pad + 8 + 8 * pairs.len()
            }
        };
        pc = pc
            .checked_add(size)
            .ok_or(Error::CodeTooLarge(usize::MAX))?;
        if pc > u32::MAX as usize {
            return Err(Error::CodeTooLarge(pc));
        }
    }
    Ok((offsets, pc))
}

// Every `ldc`-family instruction is sized as 3 bytes because we cannot know
// the final pool index during layout (it depends on interning order which is
// not fixed until emission). Always emitting the `ldc_w`/`ldc2_w` forms
// keeps `compute_layout` correct for branches and line numbers; we trade
// one byte per narrow-index `ldc` call for correctness under large pools.
// Callers that need the 1-byte narrow form can pre-intern and emit
// `Instruction::Ldc(u8)` directly via `CodeBuilder::raw`.
const LDC_SIZE: usize = 3;

fn resolve_ldc(value: &LdcValue, pool: &mut ConstantPool) -> Result<Instruction> {
    Ok(match value {
        LdcValue::Int(v) => Instruction::LdcW(pool.intern(Constant::Integer(*v))?),
        LdcValue::Float(bits) => Instruction::LdcW(pool.intern(Constant::Float(*bits))?),
        LdcValue::String(s) => Instruction::LdcW(pool.intern_string(s)?),
        LdcValue::Class(c) => Instruction::LdcW(pool.intern_class(c)?),
        LdcValue::MethodType(desc) => Instruction::LdcW(pool.intern_method_type(desc)?),
        LdcValue::MethodHandle {
            kind,
            reference_index,
        } => Instruction::LdcW(pool.intern_method_handle(*kind, *reference_index)?),
        LdcValue::Long(v) => Instruction::Ldc2W(pool.intern(Constant::Long(*v))?),
        LdcValue::Double(bits) => Instruction::Ldc2W(pool.intern(Constant::Double(*bits))?),
    })
}

fn infer_max_locals(ops: &[Op]) -> u16 {
    let mut max = 0u16;
    for op in ops {
        if let Op::Raw(ins) = op {
            let (i, wide) = match ins {
                Instruction::Iload(i)
                | Instruction::Fload(i)
                | Instruction::Aload(i)
                | Instruction::Istore(i)
                | Instruction::Fstore(i)
                | Instruction::Astore(i)
                | Instruction::Ret(i) => (u16::from(*i), false),
                Instruction::Lload(i)
                | Instruction::Dload(i)
                | Instruction::Lstore(i)
                | Instruction::Dstore(i) => (u16::from(*i), true),
                Instruction::Iload0
                | Instruction::Fload0
                | Instruction::Aload0
                | Instruction::Istore0
                | Instruction::Fstore0
                | Instruction::Astore0 => (0, false),
                Instruction::Iload1
                | Instruction::Fload1
                | Instruction::Aload1
                | Instruction::Istore1
                | Instruction::Fstore1
                | Instruction::Astore1 => (1, false),
                Instruction::Iload2
                | Instruction::Fload2
                | Instruction::Aload2
                | Instruction::Istore2
                | Instruction::Fstore2
                | Instruction::Astore2 => (2, false),
                Instruction::Iload3
                | Instruction::Fload3
                | Instruction::Aload3
                | Instruction::Istore3
                | Instruction::Fstore3
                | Instruction::Astore3 => (3, false),
                Instruction::Lload0
                | Instruction::Dload0
                | Instruction::Lstore0
                | Instruction::Dstore0 => (0, true),
                Instruction::Lload1
                | Instruction::Dload1
                | Instruction::Lstore1
                | Instruction::Dstore1 => (1, true),
                Instruction::Lload2
                | Instruction::Dload2
                | Instruction::Lstore2
                | Instruction::Dstore2 => (2, true),
                Instruction::Lload3
                | Instruction::Dload3
                | Instruction::Lstore3
                | Instruction::Dstore3 => (3, true),
                Instruction::Iinc { index, .. } => (u16::from(*index), false),
                Instruction::Wide(w) => match w {
                    WideInstruction::Iload(i)
                    | WideInstruction::Fload(i)
                    | WideInstruction::Aload(i)
                    | WideInstruction::Istore(i)
                    | WideInstruction::Fstore(i)
                    | WideInstruction::Astore(i)
                    | WideInstruction::Ret(i)
                    | WideInstruction::Iinc { index: i, .. } => (*i, false),
                    WideInstruction::Lload(i)
                    | WideInstruction::Dload(i)
                    | WideInstruction::Lstore(i)
                    | WideInstruction::Dstore(i) => (*i, true),
                },
                _ => continue,
            };
            let width = i + 1 + u16::from(wide);
            if width > max {
                max = width;
            }
        }
    }
    max
}
