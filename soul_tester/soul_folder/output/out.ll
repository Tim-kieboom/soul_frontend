; ModuleID = 'Test'
source_filename = "Test"

@_1 = constant i64 1
@_2 = constant i64 1
@_3 = constant i64 2

; Function Attrs: null_pointer_is_valid
declare void @exit(i32) #0

define void @main() {
bb_9:
  %_4 = alloca i64, align 8
  %_6 = alloca { i64, i64 }, align 8
  %_7 = alloca i64, align 8
  call void @___initGlobals()
  %exit_call = call i64 @Lib___A()
  store i64 %exit_call, ptr %_4, align 4
  %exit_call1 = call i64 @Lib___B()
  store i64 %exit_call1, ptr %_4, align 4
  %exit_call2 = call i64 @Lib___A()
  store i64 %exit_call2, ptr %_4, align 4
  %exit_call3 = call i64 @Lib___B()
  store i64 %exit_call3, ptr %_4, align 4
  %load = load i64, ptr @_2, align 4
  store i64 %load, ptr %_4, align 4
  %load4 = load i64, ptr @_2, align 4
  store i64 %load4, ptr %_4, align 4
  %load5 = load i64, ptr @_3, align 4
  store i64 %load5, ptr %_4, align 4
  %exit_call6 = call { i64, i64 } @Lib___GetStruct()
  store { i64, i64 } %exit_call6, ptr %_6, align 4
  %gep_struct = getelementptr inbounds { i64, i64 }, ptr %_6, i32 0, i32 0
  %load_field = load i64, ptr %gep_struct, align 4
  %field_tmp = alloca i64, align 8
  store i64 %load_field, ptr %field_tmp, align 4
  %source_value = load i64, ptr %field_tmp, align 4
  store i64 %source_value, ptr %_7, align 4
  ret void
}

define void @___initGlobals() {
bb_1:
  ret void
}

define i64 @Lib___A() {
bb_5:
  ret i64 1
}

define i64 @Lib___B() {
bb_6:
  ret i64 2
}

define { i64, i64 } @Lib___GetStruct() {
bb_3:
  ret { i64, i64 } { i64 1, i64 2 }
}

attributes #0 = { null_pointer_is_valid }
