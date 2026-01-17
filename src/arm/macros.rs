#[cfg(feature = "fp-simd")]
macro_rules! include_fp_asm_macros {
    () => {
        r#"
            .ifndef FP_MACROS_FLAG
            .equ FP_MACROS_FLAG, 1

            .macro VPUSH_FP_REGS base
                vstm \base!, {d0-d15}
                vstm \base!, {d16-d31}
            .endm

            .macro VPOP_FP_REGS base
                vldm \base!, {d0-d15}
                vldm \base!, {d16-d31}
            .endm

            .macro CLEAR_FP_REGS
                vmov.i32 q0, #0
                vmov.i32 q1, #0
                vmov.i32 q2, #0
                vmov.i32 q3, #0
                vmov.i32 q4, #0
                vmov.i32 q5, #0
                vmov.i32 q6, #0
                vmov.i32 q7, #0
                vmov.i32 q8, #0
                vmov.i32 q9, #0
                vmov.i32 q10, #0
                vmov.i32 q11, #0
                vmov.i32 q12, #0
                vmov.i32 q13, #0
                vmov.i32 q14, #0
                vmov.i32 q15, #0
            .endm

            .endif"#
    };
}
