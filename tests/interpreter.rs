use std::{fs, path::PathBuf, process::Command};

const BACKENDS: [(&str, &[&str]); 2] = [("tree", &[]), ("vm", &["--vm"])];
const VM: &[&str] = &["--vm"];

struct Outcome {
    stdout: String,
    stderr: String,
    code: i32,
}

fn run(name: &str, source: &str, flags: &[&str]) -> Outcome {
    let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("{name}.lox"));
    fs::write(&path, source).expect("the target temporary directory is writable");
    let output = Command::new(env!("CARGO_BIN_EXE_rustlox"))
        .args(flags)
        .arg(&path)
        .output()
        .expect("the interpreter binary runs");
    Outcome {
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        code: output
            .status
            .code()
            .expect("the interpreter exits normally"),
    }
}

fn assert_prints_with(name: &str, source: &str, flags: &[&str], expected: &[&str]) {
    let outcome = run(name, source, flags);
    assert_eq!(
        outcome.stderr, "",
        "unexpected errors for {name} with {flags:?}"
    );
    assert_eq!(outcome.code, 0);
    assert_eq!(outcome.stdout.lines().collect::<Vec<_>>(), expected);
}

fn assert_fails_with(name: &str, source: &str, flags: &[&str], code: i32, message: &str) {
    let outcome = run(name, source, flags);
    assert_eq!(
        outcome.code, code,
        "wrong exit code for {name} with {flags:?}"
    );
    assert!(
        outcome.stderr.contains(message),
        "expected '{message}' in stderr with {flags:?}, got '{}'",
        outcome.stderr
    );
}

fn assert_prints(name: &str, source: &str, expected: &[&str]) {
    for (backend, flags) in BACKENDS {
        assert_prints_with(&format!("{name}_{backend}"), source, flags, expected);
    }
}

fn assert_fails(name: &str, source: &str, code: i32, message: &str) {
    for (backend, flags) in BACKENDS {
        assert_fails_with(&format!("{name}_{backend}"), source, flags, code, message);
    }
}

#[test]
fn evaluates_expressions_with_precedence() {
    assert_prints(
        "expressions",
        "print 1 + 2 * 3 - 4 / 2; print (1 + 2) * 3; print -2.5; print 7 >= 7; print !nil;",
        &["5", "9", "-2.5", "true", "true"],
    );
}

#[test]
fn compares_values_by_type_and_content() {
    assert_prints(
        "equality",
        "print 1 == 1; print \"a\" == \"a\"; print nil == false; print 1 == \"1\"; print nil == nil;",
        &["true", "true", "false", "false", "true"],
    );
}

#[test]
fn concatenates_strings() {
    assert_prints("concat", "print \"con\" + \"cat\";", &["concat"]);
}

#[test]
fn short_circuits_logical_operators() {
    assert_prints(
        "logical",
        "print nil or \"yes\"; print false and undefined; print 1 and 2;",
        &["yes", "false", "2"],
    );
}

#[test]
fn scopes_shadow_and_restore_variables() {
    assert_prints(
        "scopes",
        "var a = 1; { var a = 2; print a; } print a; a = 3; print a;",
        &["2", "1", "3"],
    );
}

#[test]
fn runs_control_flow() {
    assert_prints(
        "control_flow",
        "for (var i = 0; i < 3; i = i + 1) { if (i == 1) print \"one\"; else print i; } \
         var n = 0; while (n < 5) n = n + 1; print n;",
        &["0", "one", "2", "5"],
    );
}

#[test]
fn supports_recursion_and_closures() {
    assert_prints(
        "functions",
        "fun fib(n) { if (n < 2) return n; return fib(n - 1) + fib(n - 2); } print fib(15); \
         fun counter() { var i = 0; fun next() { i = i + 1; return i; } return next; } \
         var c = counter(); c(); print c(); print fib; print clock;",
        &["610", "2", "<fn fib>", "<native fn>"],
    );
}

#[test]
fn binds_closures_statically() {
    assert_prints(
        "static_binding",
        "var a = \"global\"; { fun show() { print a; } show(); var a = \"block\"; show(); }",
        &["global", "global"],
    );
}

#[test]
fn supports_classes_and_inheritance() {
    assert_prints(
        "classes",
        "class A { init(x) { this.x = x; } describe() { return \"A\" + this.x; } } \
         class B < A { describe() { return \"B/\" + super.describe(); } } \
         var b = B(\"!\"); print b.describe(); print b; print B; \
         var method = b.describe; print method(); print b.init(\"?\") == b; print b.x;",
        &["B/A!", "B instance", "B", "B/A!", "true", "?"],
    );
}

#[test]
fn reports_runtime_errors() {
    assert_fails(
        "type_error",
        "print 1 - \"a\";",
        70,
        "Operands must be numbers.",
    );
    assert_fails(
        "undefined",
        "print missing;",
        70,
        "Undefined variable 'missing'.",
    );
    assert_fails(
        "arity",
        "fun f(a) {} f();",
        70,
        "Expected 1 arguments but got 0.",
    );
    assert_fails(
        "not_callable",
        "1();",
        70,
        "Can only call functions and classes.",
    );
    assert_fails(
        "property",
        "class A {} A().b;",
        70,
        "Undefined property 'b'.",
    );
}

#[test]
fn reports_static_errors() {
    assert_fails("unterminated", "print \"oops", 65, "Unterminated string.");
    assert_fails(
        "missing_semicolon",
        "print 1",
        65,
        "Expect ';' after value.",
    );
    assert_fails("assignment", "1 = 2;", 65, "Invalid assignment target.");
    assert_fails(
        "top_return",
        "return;",
        65,
        "Can't return from top-level code.",
    );
    assert_fails(
        "self_init",
        "{ var a = a; }",
        65,
        "Can't read local variable",
    );
    assert_fails(
        "this",
        "print this;",
        65,
        "Can't use 'this' outside of a class.",
    );
    assert_fails(
        "inherit",
        "class A < A {}",
        65,
        "A class can't inherit from itself.",
    );
}

#[test]
fn captures_variables_rather_than_values() {
    assert_prints(
        "capture",
        "var set; var get; \
         fun main() { var a = \"initial\"; fun s() { a = \"updated\"; } fun g() { print a; } set = s; get = g; } \
         main(); set(); get(); \
         var f; { var i = 1; fun p() { print i; } f = p; i = 2; } f(); \
         fun outer() { var x = \"deep\"; fun middle() { fun inner() { print x; } return inner; } return middle; } \
         outer()()();",
        &["updated", "2", "deep"],
    );
}

#[test]
fn invokes_fields_and_inherited_methods() {
    assert_prints(
        "invoke",
        "fun three() { return 3; } class A { name() { return \"A\"; } } \
         class B < A { greet() { return \"hi \" + super.name(); } } class C < B {} \
         var c = C(); c.field = three; print c.field(); print c.greet(); print c.name;",
        &["3", "hi A", "<fn name>"],
    );
}

#[test]
fn reclaims_garbage_while_running() {
    assert_prints(
        "churn",
        "class Node { init(value, next) { this.value = value; this.next = next; } } \
         var total = 0; \
         for (var round = 0; round < 50; round = round + 1) { \
           var list = nil; \
           for (var i = 0; i < 50; i = i + 1) { fun f() { return i; } list = Node(\"s\" + \"t\", list); list.f = f; } \
           while (list != nil) { total = total + list.f(); list = list.next; } \
         } \
         print total;",
        &["125000"],
    );
}

#[test]
fn vm_reports_stack_traces() {
    assert_fails_with(
        "trace",
        "fun a() { b(); }\nfun b() { 1 + nil; }\na();",
        VM,
        70,
        "Operands must be two numbers or two strings.\n[{2:11}] in b()\n[{1:11}] in a()\n[{3:1}] in script",
    );
}

#[test]
fn vm_detects_stack_overflow() {
    assert_fails_with(
        "overflow",
        "fun f() { f(); } f();",
        VM,
        70,
        "Stack overflow.",
    );
}

#[test]
fn vm_limits_locals_per_function() {
    let locals: String = (0..256).map(|i| format!("var a{i}; ")).collect();
    assert_fails_with(
        "locals",
        &format!("fun f() {{ {locals} }}"),
        VM,
        65,
        "Too many local variables in function.",
    );
}

#[test]
fn vm_limits_constants_per_chunk() {
    let constants: String = (0..300).map(|i| format!("print {i};")).collect();
    assert_fails_with(
        "constants",
        &constants,
        VM,
        65,
        "Too many constants in one chunk.",
    );
}

#[test]
fn disassembles_bytecode() {
    let outcome = run("disassemble", "print 1 + 2;", &["--disassemble"]);
    assert_eq!(outcome.code, 0);
    assert!(outcome.stdout.contains("== <script> =="));
    assert!(outcome.stdout.contains("Constant(0) '1'"));
    assert!(outcome.stdout.contains("Add"));
    assert!(outcome.stdout.ends_with("3\n"));
}
