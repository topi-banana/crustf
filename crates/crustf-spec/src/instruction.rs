//! JVM bytecode instructions (JVMS §6.5).
//!
//! The [`Instruction`] enum is exhaustive: every opcode defined by the
//! specification, including the reserved `breakpoint`/`impdep1`/`impdep2`
//! opcodes, has a dedicated variant. Each instruction knows how to encode
//! itself into the `code` byte array of a `Code` attribute.
//!
//! Branch instructions carry already-resolved byte offsets (`i16` for short
//! branches, `i32` for `goto_w`/`jsr_w`). Label resolution is the concern
//! of higher level crates such as `crustf-asm`.

use alloc::vec::Vec;

use crate::error::{Error, Result};

/// `newarray` atype operand (JVMS Table 6.5.newarray-A).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArrayType {
    Boolean = 4,
    Char = 5,
    Float = 6,
    Double = 7,
    Byte = 8,
    Short = 9,
    Int = 10,
    Long = 11,
}

/// Operand payload of the `wide` prefix instruction (JVMS §6.5.wide).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WideInstruction {
    Iload(u16),
    Lload(u16),
    Fload(u16),
    Dload(u16),
    Aload(u16),
    Istore(u16),
    Lstore(u16),
    Fstore(u16),
    Dstore(u16),
    Astore(u16),
    Ret(u16),
    Iinc { index: u16, delta: i16 },
}

impl WideInstruction {
    const fn inner_opcode(self) -> u8 {
        match self {
            Self::Iload(_) => 0x15,
            Self::Lload(_) => 0x16,
            Self::Fload(_) => 0x17,
            Self::Dload(_) => 0x18,
            Self::Aload(_) => 0x19,
            Self::Istore(_) => 0x36,
            Self::Lstore(_) => 0x37,
            Self::Fstore(_) => 0x38,
            Self::Dstore(_) => 0x39,
            Self::Astore(_) => 0x3a,
            Self::Ret(_) => 0xa9,
            Self::Iinc { .. } => 0x84,
        }
    }
}

/// A single decoded JVM instruction.
///
/// All variants carry JVM-native operand representations. Constant pool
/// indices are `u16`, branches are signed offsets relative to the start of
/// the instruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Instruction {
    Nop,
    AconstNull,
    IconstM1,
    Iconst0,
    Iconst1,
    Iconst2,
    Iconst3,
    Iconst4,
    Iconst5,
    Lconst0,
    Lconst1,
    Fconst0,
    Fconst1,
    Fconst2,
    Dconst0,
    Dconst1,
    Bipush(i8),
    Sipush(i16),
    Ldc(u8),
    LdcW(u16),
    Ldc2W(u16),
    Iload(u8),
    Lload(u8),
    Fload(u8),
    Dload(u8),
    Aload(u8),
    Iload0,
    Iload1,
    Iload2,
    Iload3,
    Lload0,
    Lload1,
    Lload2,
    Lload3,
    Fload0,
    Fload1,
    Fload2,
    Fload3,
    Dload0,
    Dload1,
    Dload2,
    Dload3,
    Aload0,
    Aload1,
    Aload2,
    Aload3,
    Iaload,
    Laload,
    Faload,
    Daload,
    Aaload,
    Baload,
    Caload,
    Saload,
    Istore(u8),
    Lstore(u8),
    Fstore(u8),
    Dstore(u8),
    Astore(u8),
    Istore0,
    Istore1,
    Istore2,
    Istore3,
    Lstore0,
    Lstore1,
    Lstore2,
    Lstore3,
    Fstore0,
    Fstore1,
    Fstore2,
    Fstore3,
    Dstore0,
    Dstore1,
    Dstore2,
    Dstore3,
    Astore0,
    Astore1,
    Astore2,
    Astore3,
    Iastore,
    Lastore,
    Fastore,
    Dastore,
    Aastore,
    Bastore,
    Castore,
    Sastore,
    Pop,
    Pop2,
    Dup,
    DupX1,
    DupX2,
    Dup2,
    Dup2X1,
    Dup2X2,
    Swap,
    Iadd,
    Ladd,
    Fadd,
    Dadd,
    Isub,
    Lsub,
    Fsub,
    Dsub,
    Imul,
    Lmul,
    Fmul,
    Dmul,
    Idiv,
    Ldiv,
    Fdiv,
    Ddiv,
    Irem,
    Lrem,
    Frem,
    Drem,
    Ineg,
    Lneg,
    Fneg,
    Dneg,
    Ishl,
    Lshl,
    Ishr,
    Lshr,
    Iushr,
    Lushr,
    Iand,
    Land,
    Ior,
    Lor,
    Ixor,
    Lxor,
    Iinc {
        index: u8,
        delta: i8,
    },
    I2l,
    I2f,
    I2d,
    L2i,
    L2f,
    L2d,
    F2i,
    F2l,
    F2d,
    D2i,
    D2l,
    D2f,
    I2b,
    I2c,
    I2s,
    Lcmp,
    Fcmpl,
    Fcmpg,
    Dcmpl,
    Dcmpg,
    Ifeq(i16),
    Ifne(i16),
    Iflt(i16),
    Ifge(i16),
    Ifgt(i16),
    Ifle(i16),
    IfIcmpeq(i16),
    IfIcmpne(i16),
    IfIcmplt(i16),
    IfIcmpge(i16),
    IfIcmpgt(i16),
    IfIcmple(i16),
    IfAcmpeq(i16),
    IfAcmpne(i16),
    Goto(i16),
    Jsr(i16),
    Ret(u8),
    Tableswitch {
        default: i32,
        low: i32,
        high: i32,
        offsets: Vec<i32>,
    },
    Lookupswitch {
        default: i32,
        pairs: Vec<(i32, i32)>,
    },
    Ireturn,
    Lreturn,
    Freturn,
    Dreturn,
    Areturn,
    Return,
    Getstatic(u16),
    Putstatic(u16),
    Getfield(u16),
    Putfield(u16),
    Invokevirtual(u16),
    Invokespecial(u16),
    Invokestatic(u16),
    Invokeinterface {
        index: u16,
        count: u8,
    },
    Invokedynamic(u16),
    New(u16),
    Newarray(ArrayType),
    Anewarray(u16),
    Arraylength,
    Athrow,
    Checkcast(u16),
    Instanceof(u16),
    Monitorenter,
    Monitorexit,
    Wide(WideInstruction),
    Multianewarray {
        index: u16,
        dimensions: u8,
    },
    Ifnull(i16),
    Ifnonnull(i16),
    GotoW(i32),
    JsrW(i32),
    Breakpoint,
    Impdep1,
    Impdep2,
}

impl Instruction {
    /// Opcode byte written first in the encoded form. For `wide` the outer
    /// `0xc4` byte is returned; the modified instruction uses its own opcode
    /// as a second byte.
    #[must_use]
    pub const fn opcode(&self) -> u8 {
        match self {
            Self::Nop => 0x00,
            Self::AconstNull => 0x01,
            Self::IconstM1 => 0x02,
            Self::Iconst0 => 0x03,
            Self::Iconst1 => 0x04,
            Self::Iconst2 => 0x05,
            Self::Iconst3 => 0x06,
            Self::Iconst4 => 0x07,
            Self::Iconst5 => 0x08,
            Self::Lconst0 => 0x09,
            Self::Lconst1 => 0x0a,
            Self::Fconst0 => 0x0b,
            Self::Fconst1 => 0x0c,
            Self::Fconst2 => 0x0d,
            Self::Dconst0 => 0x0e,
            Self::Dconst1 => 0x0f,
            Self::Bipush(_) => 0x10,
            Self::Sipush(_) => 0x11,
            Self::Ldc(_) => 0x12,
            Self::LdcW(_) => 0x13,
            Self::Ldc2W(_) => 0x14,
            Self::Iload(_) => 0x15,
            Self::Lload(_) => 0x16,
            Self::Fload(_) => 0x17,
            Self::Dload(_) => 0x18,
            Self::Aload(_) => 0x19,
            Self::Iload0 => 0x1a,
            Self::Iload1 => 0x1b,
            Self::Iload2 => 0x1c,
            Self::Iload3 => 0x1d,
            Self::Lload0 => 0x1e,
            Self::Lload1 => 0x1f,
            Self::Lload2 => 0x20,
            Self::Lload3 => 0x21,
            Self::Fload0 => 0x22,
            Self::Fload1 => 0x23,
            Self::Fload2 => 0x24,
            Self::Fload3 => 0x25,
            Self::Dload0 => 0x26,
            Self::Dload1 => 0x27,
            Self::Dload2 => 0x28,
            Self::Dload3 => 0x29,
            Self::Aload0 => 0x2a,
            Self::Aload1 => 0x2b,
            Self::Aload2 => 0x2c,
            Self::Aload3 => 0x2d,
            Self::Iaload => 0x2e,
            Self::Laload => 0x2f,
            Self::Faload => 0x30,
            Self::Daload => 0x31,
            Self::Aaload => 0x32,
            Self::Baload => 0x33,
            Self::Caload => 0x34,
            Self::Saload => 0x35,
            Self::Istore(_) => 0x36,
            Self::Lstore(_) => 0x37,
            Self::Fstore(_) => 0x38,
            Self::Dstore(_) => 0x39,
            Self::Astore(_) => 0x3a,
            Self::Istore0 => 0x3b,
            Self::Istore1 => 0x3c,
            Self::Istore2 => 0x3d,
            Self::Istore3 => 0x3e,
            Self::Lstore0 => 0x3f,
            Self::Lstore1 => 0x40,
            Self::Lstore2 => 0x41,
            Self::Lstore3 => 0x42,
            Self::Fstore0 => 0x43,
            Self::Fstore1 => 0x44,
            Self::Fstore2 => 0x45,
            Self::Fstore3 => 0x46,
            Self::Dstore0 => 0x47,
            Self::Dstore1 => 0x48,
            Self::Dstore2 => 0x49,
            Self::Dstore3 => 0x4a,
            Self::Astore0 => 0x4b,
            Self::Astore1 => 0x4c,
            Self::Astore2 => 0x4d,
            Self::Astore3 => 0x4e,
            Self::Iastore => 0x4f,
            Self::Lastore => 0x50,
            Self::Fastore => 0x51,
            Self::Dastore => 0x52,
            Self::Aastore => 0x53,
            Self::Bastore => 0x54,
            Self::Castore => 0x55,
            Self::Sastore => 0x56,
            Self::Pop => 0x57,
            Self::Pop2 => 0x58,
            Self::Dup => 0x59,
            Self::DupX1 => 0x5a,
            Self::DupX2 => 0x5b,
            Self::Dup2 => 0x5c,
            Self::Dup2X1 => 0x5d,
            Self::Dup2X2 => 0x5e,
            Self::Swap => 0x5f,
            Self::Iadd => 0x60,
            Self::Ladd => 0x61,
            Self::Fadd => 0x62,
            Self::Dadd => 0x63,
            Self::Isub => 0x64,
            Self::Lsub => 0x65,
            Self::Fsub => 0x66,
            Self::Dsub => 0x67,
            Self::Imul => 0x68,
            Self::Lmul => 0x69,
            Self::Fmul => 0x6a,
            Self::Dmul => 0x6b,
            Self::Idiv => 0x6c,
            Self::Ldiv => 0x6d,
            Self::Fdiv => 0x6e,
            Self::Ddiv => 0x6f,
            Self::Irem => 0x70,
            Self::Lrem => 0x71,
            Self::Frem => 0x72,
            Self::Drem => 0x73,
            Self::Ineg => 0x74,
            Self::Lneg => 0x75,
            Self::Fneg => 0x76,
            Self::Dneg => 0x77,
            Self::Ishl => 0x78,
            Self::Lshl => 0x79,
            Self::Ishr => 0x7a,
            Self::Lshr => 0x7b,
            Self::Iushr => 0x7c,
            Self::Lushr => 0x7d,
            Self::Iand => 0x7e,
            Self::Land => 0x7f,
            Self::Ior => 0x80,
            Self::Lor => 0x81,
            Self::Ixor => 0x82,
            Self::Lxor => 0x83,
            Self::Iinc { .. } => 0x84,
            Self::I2l => 0x85,
            Self::I2f => 0x86,
            Self::I2d => 0x87,
            Self::L2i => 0x88,
            Self::L2f => 0x89,
            Self::L2d => 0x8a,
            Self::F2i => 0x8b,
            Self::F2l => 0x8c,
            Self::F2d => 0x8d,
            Self::D2i => 0x8e,
            Self::D2l => 0x8f,
            Self::D2f => 0x90,
            Self::I2b => 0x91,
            Self::I2c => 0x92,
            Self::I2s => 0x93,
            Self::Lcmp => 0x94,
            Self::Fcmpl => 0x95,
            Self::Fcmpg => 0x96,
            Self::Dcmpl => 0x97,
            Self::Dcmpg => 0x98,
            Self::Ifeq(_) => 0x99,
            Self::Ifne(_) => 0x9a,
            Self::Iflt(_) => 0x9b,
            Self::Ifge(_) => 0x9c,
            Self::Ifgt(_) => 0x9d,
            Self::Ifle(_) => 0x9e,
            Self::IfIcmpeq(_) => 0x9f,
            Self::IfIcmpne(_) => 0xa0,
            Self::IfIcmplt(_) => 0xa1,
            Self::IfIcmpge(_) => 0xa2,
            Self::IfIcmpgt(_) => 0xa3,
            Self::IfIcmple(_) => 0xa4,
            Self::IfAcmpeq(_) => 0xa5,
            Self::IfAcmpne(_) => 0xa6,
            Self::Goto(_) => 0xa7,
            Self::Jsr(_) => 0xa8,
            Self::Ret(_) => 0xa9,
            Self::Tableswitch { .. } => 0xaa,
            Self::Lookupswitch { .. } => 0xab,
            Self::Ireturn => 0xac,
            Self::Lreturn => 0xad,
            Self::Freturn => 0xae,
            Self::Dreturn => 0xaf,
            Self::Areturn => 0xb0,
            Self::Return => 0xb1,
            Self::Getstatic(_) => 0xb2,
            Self::Putstatic(_) => 0xb3,
            Self::Getfield(_) => 0xb4,
            Self::Putfield(_) => 0xb5,
            Self::Invokevirtual(_) => 0xb6,
            Self::Invokespecial(_) => 0xb7,
            Self::Invokestatic(_) => 0xb8,
            Self::Invokeinterface { .. } => 0xb9,
            Self::Invokedynamic(_) => 0xba,
            Self::New(_) => 0xbb,
            Self::Newarray(_) => 0xbc,
            Self::Anewarray(_) => 0xbd,
            Self::Arraylength => 0xbe,
            Self::Athrow => 0xbf,
            Self::Checkcast(_) => 0xc0,
            Self::Instanceof(_) => 0xc1,
            Self::Monitorenter => 0xc2,
            Self::Monitorexit => 0xc3,
            Self::Wide(_) => 0xc4,
            Self::Multianewarray { .. } => 0xc5,
            Self::Ifnull(_) => 0xc6,
            Self::Ifnonnull(_) => 0xc7,
            Self::GotoW(_) => 0xc8,
            Self::JsrW(_) => 0xc9,
            Self::Breakpoint => 0xca,
            Self::Impdep1 => 0xfe,
            Self::Impdep2 => 0xff,
        }
    }

    /// Size in bytes of the encoded instruction starting at `offset` within
    /// the enclosing `Code` array. The offset is only consulted for
    /// `tableswitch`/`lookupswitch`, which require 0-3 padding bytes so the
    /// first 32-bit operand lies on a 4 byte boundary (JVMS §6.5.tableswitch).
    #[must_use]
    pub fn size(&self, offset: usize) -> usize {
        match self {
            Self::Bipush(_)
            | Self::Ldc(_)
            | Self::Iload(_)
            | Self::Lload(_)
            | Self::Fload(_)
            | Self::Dload(_)
            | Self::Aload(_)
            | Self::Istore(_)
            | Self::Lstore(_)
            | Self::Fstore(_)
            | Self::Dstore(_)
            | Self::Astore(_)
            | Self::Ret(_)
            | Self::Newarray(_) => 2,
            Self::Sipush(_)
            | Self::LdcW(_)
            | Self::Ldc2W(_)
            | Self::Iinc { .. }
            | Self::Ifeq(_)
            | Self::Ifne(_)
            | Self::Iflt(_)
            | Self::Ifge(_)
            | Self::Ifgt(_)
            | Self::Ifle(_)
            | Self::IfIcmpeq(_)
            | Self::IfIcmpne(_)
            | Self::IfIcmplt(_)
            | Self::IfIcmpge(_)
            | Self::IfIcmpgt(_)
            | Self::IfIcmple(_)
            | Self::IfAcmpeq(_)
            | Self::IfAcmpne(_)
            | Self::Goto(_)
            | Self::Jsr(_)
            | Self::Getstatic(_)
            | Self::Putstatic(_)
            | Self::Getfield(_)
            | Self::Putfield(_)
            | Self::Invokevirtual(_)
            | Self::Invokespecial(_)
            | Self::Invokestatic(_)
            | Self::New(_)
            | Self::Anewarray(_)
            | Self::Checkcast(_)
            | Self::Instanceof(_)
            | Self::Ifnull(_)
            | Self::Ifnonnull(_) => 3,
            Self::Multianewarray { .. } => 4,
            Self::Invokeinterface { .. }
            | Self::Invokedynamic(_)
            | Self::GotoW(_)
            | Self::JsrW(_) => 5,
            Self::Wide(w) => match w {
                WideInstruction::Iinc { .. } => 6,
                _ => 4,
            },
            Self::Tableswitch { low, high, .. } => {
                let pad = (4 - ((offset + 1) % 4)) % 4;
                1 + pad + 12 + 4 * ((high - low + 1) as usize)
            }
            Self::Lookupswitch { pairs, .. } => {
                let pad = (4 - ((offset + 1) % 4)) % 4;
                1 + pad + 8 + 8 * pairs.len()
            }
            _ => 1,
        }
    }

    /// Append the encoded bytes of this instruction to `out`. The starting
    /// offset is the current `out.len()` (minus any bytes outside the code
    /// array the caller may have in the buffer; pass a dedicated buffer).
    pub fn encode(&self, out: &mut Vec<u8>) -> Result<()> {
        let offset = out.len();
        out.push(self.opcode());
        match self {
            Self::Bipush(b) => out.push(*b as u8),
            Self::Sipush(s) => out.extend_from_slice(&s.to_be_bytes()),
            Self::Ldc(i) => out.push(*i),
            Self::LdcW(i) | Self::Ldc2W(i) => out.extend_from_slice(&i.to_be_bytes()),
            Self::Iload(i)
            | Self::Lload(i)
            | Self::Fload(i)
            | Self::Dload(i)
            | Self::Aload(i)
            | Self::Istore(i)
            | Self::Lstore(i)
            | Self::Fstore(i)
            | Self::Dstore(i)
            | Self::Astore(i)
            | Self::Ret(i) => out.push(*i),
            Self::Iinc { index, delta } => {
                out.push(*index);
                out.push(*delta as u8);
            }
            Self::Ifeq(b)
            | Self::Ifne(b)
            | Self::Iflt(b)
            | Self::Ifge(b)
            | Self::Ifgt(b)
            | Self::Ifle(b)
            | Self::IfIcmpeq(b)
            | Self::IfIcmpne(b)
            | Self::IfIcmplt(b)
            | Self::IfIcmpge(b)
            | Self::IfIcmpgt(b)
            | Self::IfIcmple(b)
            | Self::IfAcmpeq(b)
            | Self::IfAcmpne(b)
            | Self::Goto(b)
            | Self::Jsr(b)
            | Self::Ifnull(b)
            | Self::Ifnonnull(b) => out.extend_from_slice(&b.to_be_bytes()),
            Self::Tableswitch {
                default,
                low,
                high,
                offsets,
            } => {
                let pad = (4 - ((offset + 1) % 4)) % 4;
                for _ in 0..pad {
                    out.push(0);
                }
                out.extend_from_slice(&default.to_be_bytes());
                out.extend_from_slice(&low.to_be_bytes());
                out.extend_from_slice(&high.to_be_bytes());
                if offsets.len() as i64 != (*high as i64 - *low as i64 + 1) {
                    return Err(Error::Encoding(alloc::format!(
                        "tableswitch offsets length {} != high-low+1",
                        offsets.len()
                    )));
                }
                for o in offsets {
                    out.extend_from_slice(&o.to_be_bytes());
                }
            }
            Self::Lookupswitch { default, pairs } => {
                let pad = (4 - ((offset + 1) % 4)) % 4;
                for _ in 0..pad {
                    out.push(0);
                }
                out.extend_from_slice(&default.to_be_bytes());
                out.extend_from_slice(&(pairs.len() as u32).to_be_bytes());
                for (m, off) in pairs {
                    out.extend_from_slice(&m.to_be_bytes());
                    out.extend_from_slice(&off.to_be_bytes());
                }
            }
            Self::Getstatic(i)
            | Self::Putstatic(i)
            | Self::Getfield(i)
            | Self::Putfield(i)
            | Self::Invokevirtual(i)
            | Self::Invokespecial(i)
            | Self::Invokestatic(i)
            | Self::New(i)
            | Self::Anewarray(i)
            | Self::Checkcast(i)
            | Self::Instanceof(i) => out.extend_from_slice(&i.to_be_bytes()),
            Self::Invokeinterface { index, count } => {
                out.extend_from_slice(&index.to_be_bytes());
                out.push(*count);
                out.push(0);
            }
            Self::Invokedynamic(i) => {
                out.extend_from_slice(&i.to_be_bytes());
                out.push(0);
                out.push(0);
            }
            Self::Newarray(a) => out.push(*a as u8),
            Self::Multianewarray { index, dimensions } => {
                out.extend_from_slice(&index.to_be_bytes());
                out.push(*dimensions);
            }
            Self::GotoW(o) | Self::JsrW(o) => out.extend_from_slice(&o.to_be_bytes()),
            Self::Wide(w) => {
                out.push(w.inner_opcode());
                match w {
                    WideInstruction::Iload(i)
                    | WideInstruction::Lload(i)
                    | WideInstruction::Fload(i)
                    | WideInstruction::Dload(i)
                    | WideInstruction::Aload(i)
                    | WideInstruction::Istore(i)
                    | WideInstruction::Lstore(i)
                    | WideInstruction::Fstore(i)
                    | WideInstruction::Dstore(i)
                    | WideInstruction::Astore(i)
                    | WideInstruction::Ret(i) => out.extend_from_slice(&i.to_be_bytes()),
                    WideInstruction::Iinc { index, delta } => {
                        out.extend_from_slice(&index.to_be_bytes());
                        out.extend_from_slice(&delta.to_be_bytes());
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opcode_is_consistent_with_size() {
        let cases: &[(Instruction, usize)] = &[
            (Instruction::Nop, 1),
            (Instruction::Bipush(5), 2),
            (Instruction::Sipush(500), 3),
            (Instruction::Iinc { index: 1, delta: 1 }, 3),
            (
                Instruction::Multianewarray {
                    index: 1,
                    dimensions: 2,
                },
                4,
            ),
            (Instruction::Invokeinterface { index: 1, count: 1 }, 5),
            (Instruction::Invokedynamic(1), 5),
            (Instruction::GotoW(0), 5),
            (Instruction::Wide(WideInstruction::Iload(100)), 4),
            (
                Instruction::Wide(WideInstruction::Iinc { index: 1, delta: 1 }),
                6,
            ),
        ];
        for (ins, expected) in cases {
            assert_eq!(ins.size(0), *expected, "{ins:?}");
        }
    }

    #[test]
    fn tableswitch_padding_depends_on_offset() {
        let ts = Instruction::Tableswitch {
            default: 0,
            low: 0,
            high: 1,
            offsets: alloc::vec![10, 20],
        };
        // opcode + pad + 3*4 (default, low, high) + 2*4 (offsets)
        // offset 0: pad=3, total=1+3+12+8=24
        assert_eq!(ts.size(0), 24);
        // offset 3: pad=0, total=1+0+12+8=21
        assert_eq!(ts.size(3), 21);
    }

    #[test]
    fn encode_bipush() {
        let mut buf = Vec::new();
        Instruction::Bipush(-3).encode(&mut buf).unwrap();
        assert_eq!(buf, [0x10, 0xfd]);
    }

    #[test]
    fn encode_invokevirtual() {
        let mut buf = Vec::new();
        Instruction::Invokevirtual(0x1234).encode(&mut buf).unwrap();
        assert_eq!(buf, [0xb6, 0x12, 0x34]);
    }

    #[test]
    fn encode_invokeinterface_trailing_zero() {
        let mut buf = Vec::new();
        Instruction::Invokeinterface {
            index: 0x0102,
            count: 3,
        }
        .encode(&mut buf)
        .unwrap();
        assert_eq!(buf, [0xb9, 0x01, 0x02, 0x03, 0x00]);
    }

    #[test]
    fn encode_tableswitch_pads_to_four() {
        // First emit a nop so the tableswitch sits at offset 1.
        let mut buf = Vec::new();
        Instruction::Nop.encode(&mut buf).unwrap();
        let ts = Instruction::Tableswitch {
            default: 0x0a,
            low: 0,
            high: 1,
            offsets: alloc::vec![0x14, 0x1e],
        };
        ts.encode(&mut buf).unwrap();
        // nop + 0xaa + 2 padding bytes (offset 1+1=2, need 2 bytes) + 4*5 bytes.
        assert_eq!(buf[0], 0x00);
        assert_eq!(buf[1], 0xaa);
        assert_eq!(&buf[2..4], &[0, 0]);
        assert_eq!(&buf[4..8], &0x0a_i32.to_be_bytes());
    }

    #[test]
    fn encode_wide_iinc() {
        let mut buf = Vec::new();
        Instruction::Wide(WideInstruction::Iinc {
            index: 0x0102,
            delta: -1,
        })
        .encode(&mut buf)
        .unwrap();
        assert_eq!(buf, [0xc4, 0x84, 0x01, 0x02, 0xff, 0xff]);
    }

    /// Exhaustively check that the first byte of every encoded instruction
    /// matches its declared opcode. This catches copy-paste bugs in either
    /// `opcode()` or `encode()`.
    #[test]
    fn every_opcode_round_trips_to_its_leading_byte() {
        use Instruction::*;
        let cases: alloc::vec::Vec<Instruction> = alloc::vec![
            Nop,
            AconstNull,
            IconstM1,
            Iconst0,
            Iconst1,
            Iconst2,
            Iconst3,
            Iconst4,
            Iconst5,
            Lconst0,
            Lconst1,
            Fconst0,
            Fconst1,
            Fconst2,
            Dconst0,
            Dconst1,
            Bipush(0),
            Sipush(0),
            Ldc(0),
            LdcW(0),
            Ldc2W(0),
            Iload(0),
            Lload(0),
            Fload(0),
            Dload(0),
            Aload(0),
            Iload0,
            Iload1,
            Iload2,
            Iload3,
            Lload0,
            Lload1,
            Lload2,
            Lload3,
            Fload0,
            Fload1,
            Fload2,
            Fload3,
            Dload0,
            Dload1,
            Dload2,
            Dload3,
            Aload0,
            Aload1,
            Aload2,
            Aload3,
            Iaload,
            Laload,
            Faload,
            Daload,
            Aaload,
            Baload,
            Caload,
            Saload,
            Istore(0),
            Lstore(0),
            Fstore(0),
            Dstore(0),
            Astore(0),
            Istore0,
            Istore1,
            Istore2,
            Istore3,
            Lstore0,
            Lstore1,
            Lstore2,
            Lstore3,
            Fstore0,
            Fstore1,
            Fstore2,
            Fstore3,
            Dstore0,
            Dstore1,
            Dstore2,
            Dstore3,
            Astore0,
            Astore1,
            Astore2,
            Astore3,
            Iastore,
            Lastore,
            Fastore,
            Dastore,
            Aastore,
            Bastore,
            Castore,
            Sastore,
            Pop,
            Pop2,
            Dup,
            DupX1,
            DupX2,
            Dup2,
            Dup2X1,
            Dup2X2,
            Swap,
            Iadd,
            Ladd,
            Fadd,
            Dadd,
            Isub,
            Lsub,
            Fsub,
            Dsub,
            Imul,
            Lmul,
            Fmul,
            Dmul,
            Idiv,
            Ldiv,
            Fdiv,
            Ddiv,
            Irem,
            Lrem,
            Frem,
            Drem,
            Ineg,
            Lneg,
            Fneg,
            Dneg,
            Ishl,
            Lshl,
            Ishr,
            Lshr,
            Iushr,
            Lushr,
            Iand,
            Land,
            Ior,
            Lor,
            Ixor,
            Lxor,
            Iinc { index: 0, delta: 0 },
            I2l,
            I2f,
            I2d,
            L2i,
            L2f,
            L2d,
            F2i,
            F2l,
            F2d,
            D2i,
            D2l,
            D2f,
            I2b,
            I2c,
            I2s,
            Lcmp,
            Fcmpl,
            Fcmpg,
            Dcmpl,
            Dcmpg,
            Ifeq(0),
            Ifne(0),
            Iflt(0),
            Ifge(0),
            Ifgt(0),
            Ifle(0),
            IfIcmpeq(0),
            IfIcmpne(0),
            IfIcmplt(0),
            IfIcmpge(0),
            IfIcmpgt(0),
            IfIcmple(0),
            IfAcmpeq(0),
            IfAcmpne(0),
            Goto(0),
            Jsr(0),
            Ret(0),
            Tableswitch {
                default: 0,
                low: 0,
                high: 0,
                offsets: alloc::vec![0]
            },
            Lookupswitch {
                default: 0,
                pairs: alloc::vec![]
            },
            Ireturn,
            Lreturn,
            Freturn,
            Dreturn,
            Areturn,
            Return,
            Getstatic(0),
            Putstatic(0),
            Getfield(0),
            Putfield(0),
            Invokevirtual(0),
            Invokespecial(0),
            Invokestatic(0),
            Invokeinterface { index: 0, count: 1 },
            Invokedynamic(0),
            New(0),
            Newarray(ArrayType::Int),
            Anewarray(0),
            Arraylength,
            Athrow,
            Checkcast(0),
            Instanceof(0),
            Monitorenter,
            Monitorexit,
            Wide(WideInstruction::Iload(0)),
            Multianewarray {
                index: 0,
                dimensions: 1
            },
            Ifnull(0),
            Ifnonnull(0),
            GotoW(0),
            JsrW(0),
            Breakpoint,
            Impdep1,
            Impdep2,
        ];
        for ins in &cases {
            let mut buf = Vec::new();
            ins.encode(&mut buf).unwrap();
            assert_eq!(buf[0], ins.opcode(), "opcode mismatch for {ins:?}");
            assert_eq!(buf.len(), ins.size(0), "size mismatch for {ins:?}");
        }
    }

    /// Covers every `wide`-modified operand (load/store/ret and iinc).
    #[test]
    fn wide_opcodes_all_emit_0xc4_prefix() {
        use WideInstruction::*;
        let cases: alloc::vec::Vec<WideInstruction> = alloc::vec![
            Iload(0),
            Lload(0),
            Fload(0),
            Dload(0),
            Aload(0),
            Istore(0),
            Lstore(0),
            Fstore(0),
            Dstore(0),
            Astore(0),
            Ret(0),
            Iinc { index: 0, delta: 0 },
        ];
        for w in &cases {
            let mut buf = Vec::new();
            Instruction::Wide(*w).encode(&mut buf).unwrap();
            assert_eq!(buf[0], 0xc4);
            assert!(buf.len() == 4 || buf.len() == 6);
        }
    }
}
