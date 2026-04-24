; ModuleID = 'Std'
source_filename = "Std"

; Function Attrs: null_pointer_is_valid
declare void @exit(i32) #0

define void @stdout_Io___WriteChar___t_mut_Stdout(i8 %0) {
bb_17:
  %_35 = alloca i8, align 1
  store i8 %0, ptr %_35, align 1
  %load = load i8, ptr %_35, align 1
  call void @__clib_printChar(i8 %load)
  ret void
}

declare void @__clib_printChar(i8)

define void @stdout_Io___WriteCstr___t_mut_Stdout(ptr %0) {
bb_18:
  %_36 = alloca ptr, align 8
  store ptr %0, ptr %_36, align 8
  %load = load ptr, ptr %_36, align 8
  call void @__clib_printCStr(ptr %load)
  ret void
}

declare void @__clib_printCStr(ptr)

define void @stdout_Io___WriteStr___t_mut_Stdout({ ptr, i64 } %0) {
bb_19:
  %_37 = alloca { ptr, i64 }, align 8
  store { ptr, i64 } %0, ptr %_37, align 8
  %load = load { ptr, i64 }, ptr %_37, align 8
  %temp_base_ptr = alloca { ptr, i64 }, align 8
  store { ptr, i64 } %load, ptr %temp_base_ptr, align 8
  %array_data_ptr = getelementptr inbounds { ptr, i64 }, ptr %temp_base_ptr, i32 0, i32 0
  %array_data = load ptr, ptr %array_data_ptr, align 8
  %gep_struct = getelementptr inbounds { ptr, i64 }, ptr %_37, i32 0, i32 1
  %load_field = load i64, ptr %gep_struct, align 4
  %field_tmp = alloca i64, align 8
  store i64 %load_field, ptr %field_tmp, align 4
  %arg_load = load i64, ptr %field_tmp, align 4
  call void @__clib_printSoulStr(ptr %array_data, i64 %arg_load)
  ret void
}

declare void @__clib_printSoulStr(ptr, i64)

define ptr @Fmt_Io___FmtUint(i64 %0, i8 %1, { ptr, i64 } %2, i1 %3) {
bb_23:
  %_51 = alloca i64, align 8
  store i64 %0, ptr %_51, align 4
  %_52 = alloca i8, align 1
  store i8 %1, ptr %_52, align 1
  %_53 = alloca { ptr, i64 }, align 8
  store { ptr, i64 } %2, ptr %_53, align 8
  %_54 = alloca i1, align 1
  store i1 %3, ptr %_54, align 1
  %load = load i64, ptr %_51, align 4
  %cast_uint_turnc = trunc i64 %load to i32
  %load1 = load { ptr, i64 }, ptr %_53, align 8
  %temp_base_ptr = alloca { ptr, i64 }, align 8
  store { ptr, i64 } %load1, ptr %temp_base_ptr, align 8
  %array_data_ptr = getelementptr inbounds { ptr, i64 }, ptr %temp_base_ptr, i32 0, i32 0
  %array_data = load ptr, ptr %array_data_ptr, align 8
  %load2 = load i8, ptr %_52, align 1
  %load3 = load i1, ptr %_54, align 1
  %exit_call = call ptr @__clib_fmtUint(i32 %cast_uint_turnc, i8 %load2, ptr %array_data, i1 %load3)
  ret ptr %exit_call
}

declare ptr @__clib_fmtUint(i32, i8, ptr, i1)

define ptr @Fmt_Io___FmtInt(i64 %0, i8 %1, { ptr, i64 } %2, i1 %3) {
bb_24:
  %_55 = alloca i64, align 8
  store i64 %0, ptr %_55, align 4
  %_56 = alloca i8, align 1
  store i8 %1, ptr %_56, align 1
  %_57 = alloca { ptr, i64 }, align 8
  store { ptr, i64 } %2, ptr %_57, align 8
  %_58 = alloca i1, align 1
  store i1 %3, ptr %_58, align 1
  %load = load i64, ptr %_55, align 4
  %cast_uint_turnc = trunc i64 %load to i32
  %load1 = load { ptr, i64 }, ptr %_57, align 8
  %temp_base_ptr = alloca { ptr, i64 }, align 8
  store { ptr, i64 } %load1, ptr %temp_base_ptr, align 8
  %array_data_ptr = getelementptr inbounds { ptr, i64 }, ptr %temp_base_ptr, i32 0, i32 0
  %array_data = load ptr, ptr %array_data_ptr, align 8
  %load2 = load i8, ptr %_56, align 1
  %load3 = load i1, ptr %_58, align 1
  %exit_call = call ptr @__clib_fmtInt(i32 %cast_uint_turnc, i8 %load2, ptr %array_data, i1 %load3)
  ret ptr %exit_call
}

declare ptr @__clib_fmtInt(i32, i8, ptr, i1)

define ptr @Fmt_Io___FmtFloat(double %0, i8 %1, { ptr, i64 } %2, i8 %3, i1 %4) {
bb_25:
  %_59 = alloca double, align 8
  store double %0, ptr %_59, align 8
  %_60 = alloca i8, align 1
  store i8 %1, ptr %_60, align 1
  %_61 = alloca { ptr, i64 }, align 8
  store { ptr, i64 } %2, ptr %_61, align 8
  %_62 = alloca i8, align 1
  store i8 %3, ptr %_62, align 1
  %_63 = alloca i1, align 1
  store i1 %4, ptr %_63, align 1
  %load = load { ptr, i64 }, ptr %_61, align 8
  %temp_base_ptr = alloca { ptr, i64 }, align 8
  store { ptr, i64 } %load, ptr %temp_base_ptr, align 8
  %array_data_ptr = getelementptr inbounds { ptr, i64 }, ptr %temp_base_ptr, i32 0, i32 0
  %array_data = load ptr, ptr %array_data_ptr, align 8
  %load1 = load double, ptr %_59, align 8
  %load2 = load i8, ptr %_60, align 1
  %load3 = load i8, ptr %_62, align 1
  %load4 = load i1, ptr %_63, align 1
  %exit_call = call ptr @__clib_fmtFloat(double %load1, i8 %load2, ptr %array_data, i8 %load3, i1 %load4)
  ret ptr %exit_call
}

declare ptr @__clib_fmtFloat(double, i8, ptr, i8, i1)

attributes #0 = { null_pointer_is_valid }
