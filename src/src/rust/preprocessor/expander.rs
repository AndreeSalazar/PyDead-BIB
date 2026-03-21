// ============================================================
// Macro Expander — C++11-C++17 → C++98 Canon
// ============================================================
// Expande syntax sugar de C++11-C++17 a C++98 internamente.
// Lambda → struct, auto → tipo inferido, range-for → iterator.
// ============================================================

/// Expande C++11-C++17 features a C++98 equivalentes
pub struct MacroExpander {
    /// Contador para generar nombres unicos (lambdas, etc.)
    counter: u32,
}

impl MacroExpander {
    pub fn new() -> Self {
        Self { counter: 0 }
    }

    /// Genera un nombre unico para estructuras generadas
    fn unique_name(&mut self, prefix: &str) -> String {
        self.counter += 1;
        format!("__{}_{}__", prefix, self.counter)
    }

    /// Expande una lambda C++11 a un struct C++98 con operator()
    ///
    /// Input:  `[](int x) { return x + 1; }`
    /// Output: `struct __lambda_1__ { int operator()(int x) const { return x + 1; } };`
    pub fn expand_lambda(&mut self, params: &str, body: &str) -> String {
        let name = self.unique_name("lambda");
        format!(
            "struct {} {{ auto operator()({}) const {{ {} }} }};",
            name, params, body
        )
    }

    /// Expande range-for C++11 a iterator C++98
    ///
    /// Input:  `for (auto& item : lista) { ... }`
    /// Output: `for (auto it = lista.begin(); it != lista.end(); ++it) { auto& item = *it; ... }`
    pub fn expand_range_for(&self, var_name: &str, container: &str, body: &str) -> String {
        format!(
            "for (auto __it = {container}.begin(); __it != {container}.end(); ++__it) {{ auto& {var} = *__it; {body} }}",
            container = container,
            var = var_name,
            body = body
        )
    }

    /// Expande if constexpr C++17 — evalua en compilacion, solo incluye branch correcto
    pub fn expand_if_constexpr(
        &self,
        condition_is_true: bool,
        then_body: &str,
        else_body: Option<&str>,
    ) -> String {
        if condition_is_true {
            then_body.to_string()
        } else {
            else_body.unwrap_or("").to_string()
        }
    }

    /// Retorna cuantas expansiones se han realizado
    pub fn expansion_count(&self) -> u32 {
        self.counter
    }

    // ============================================================
    // C++11 Features
    // ============================================================

    /// Expande `auto x = expr;` → `TYPE x = expr;`
    /// El tipo se infiere del contexto (para la expansion, el tipo concreto lo provee el caller)
    pub fn expand_auto_var(&self, var_name: &str, inferred_type: &str, init_expr: &str) -> String {
        format!("{} {} = {};", inferred_type, var_name, init_expr)
    }

    /// Expande `nullptr` → `((void*)0)` para C++98 compatibility
    pub fn expand_nullptr(&self) -> String {
        "((void*)0)".to_string()
    }

    /// Expande `static_assert(cond, msg)` → compile-time check
    /// En C++98 canon: se evalua en compilacion, si falla → error
    pub fn expand_static_assert(&self, condition: bool, message: &str) -> Result<String, String> {
        if condition {
            Ok(format!("/* static_assert passed: {} */", message))
        } else {
            Err(format!("static_assert failed: {}", message))
        }
    }

    /// Expande `enum class Name { A, B, C }` → `struct Name { enum _inner { A, B, C }; };`
    pub fn expand_enum_class(&mut self, name: &str, variants: &[&str]) -> String {
        let variants_str = variants.join(", ");
        format!(
            "struct {} {{ enum _inner {{ {} }}; typedef _inner type; }};",
            name, variants_str
        )
    }

    /// Expande `using Alias = Type;` → `typedef Type Alias;`
    pub fn expand_using_alias(&self, alias: &str, target_type: &str) -> String {
        format!("typedef {} {};", target_type, alias)
    }

    /// Expande variadic templates (parameter pack) conceptualmente
    /// `template<typename... Args> void f(Args... args)`
    /// → genera una version para cada instanciacion usada
    pub fn expand_variadic_template(
        &mut self,
        func_name: &str,
        concrete_types: &[&str],
        body: &str,
    ) -> String {
        let params: Vec<String> = concrete_types
            .iter()
            .enumerate()
            .map(|(i, t)| format!("{} __arg{}", t, i))
            .collect();
        let param_str = params.join(", ");
        let mangled = format!(
            "__variadic_{}_{}__",
            func_name,
            concrete_types.join("_")
        );
        format!("void {}({}) {{ {} }}", mangled, param_str, body)
    }

    /// Expande `constexpr int f() { return 42; }` → evaluacion en tiempo de compilacion
    pub fn expand_constexpr_func(
        &self,
        return_type: &str,
        func_name: &str,
        value: &str,
    ) -> String {
        format!(
            "/* constexpr evaluated */ static const {} {} = {};",
            return_type, func_name, value
        )
    }

    /// Expande move semantics: `std::move(x)` → transferencia de ownership
    /// En C++98 canon: simplemente copia (sin RVO/move, el optimizer maneja)
    pub fn expand_move(&self, expr: &str) -> String {
        format!("/* std::move */ {}", expr)
    }

    /// Expande `std::initializer_list<T>{a, b, c}` → array temporal
    pub fn expand_initializer_list(
        &mut self,
        element_type: &str,
        elements: &[&str],
    ) -> String {
        let name = self.unique_name("init_list");
        let elems = elements.join(", ");
        format!(
            "{} {}[] = {{ {} }};",
            element_type, name, elems
        )
    }

    /// Expande delegating constructor: `Foo() : Foo(0)` → call al otro constructor
    pub fn expand_delegating_constructor(
        &self,
        class_name: &str,
        delegate_args: &str,
    ) -> String {
        format!(
            "{}::__init__({});",
            class_name, delegate_args
        )
    }

    // ============================================================
    // C++14 Features
    // ============================================================

    /// Expande generic lambda C++14: `[](auto x) { return x; }`
    /// → struct con template operator()
    pub fn expand_generic_lambda(
        &mut self,
        params: &[(&str, &str)],  // (inferred_type, param_name)
        body: &str,
    ) -> String {
        let name = self.unique_name("generic_lambda");
        let param_strs: Vec<String> = params
            .iter()
            .map(|(ty, nm)| format!("{} {}", ty, nm))
            .collect();
        format!(
            "struct {} {{ auto operator()({}) const {{ {} }} }};",
            name,
            param_strs.join(", "),
            body
        )
    }

    /// Expande `[[deprecated("msg")]]` → marca la funcion como deprecated
    pub fn expand_deprecated(&self, func_name: &str, message: &str) -> String {
        format!(
            "/* [[deprecated(\"{}\")]] */ /* {} is deprecated */",
            message, func_name
        )
    }

    /// Expande binary literals C++14: `0b1010` → `10`
    pub fn expand_binary_literal(&self, binary_str: &str) -> Result<String, String> {
        let clean = binary_str.trim_start_matches("0b").trim_start_matches("0B");
        match i64::from_str_radix(clean, 2) {
            Ok(val) => Ok(val.to_string()),
            Err(_) => Err(format!("Invalid binary literal: {}", binary_str)),
        }
    }

    /// Expande digit separators C++14: `1'000'000` → `1000000`
    pub fn expand_digit_separator(&self, literal: &str) -> String {
        literal.replace('\'', "")
    }

    /// Expande return type deduction C++14: `auto f() { return 42; }` → `int f() { return 42; }`
    pub fn expand_return_type_deduction(
        &self,
        func_name: &str,
        inferred_return: &str,
        params: &str,
        body: &str,
    ) -> String {
        format!("{} {}({}) {{ {} }}", inferred_return, func_name, params, body)
    }

    /// Expande `std::make_unique<T>(args)` → `new T(args)` (sin smart pointer en C++98)
    pub fn expand_make_unique(&self, type_name: &str, args: &str) -> String {
        format!("new {}({})", type_name, args)
    }

    // ============================================================
    // C++17 Features
    // ============================================================

    /// Expande structured bindings: `auto [a, b] = pair;`
    /// → `auto __sb = pair; auto& a = __sb.first; auto& b = __sb.second;`
    pub fn expand_structured_binding(
        &mut self,
        names: &[&str],
        source_expr: &str,
        field_accessors: &[&str],
    ) -> String {
        let temp = self.unique_name("sb");
        let mut result = format!("auto {} = {}; ", temp, source_expr);
        for (name, accessor) in names.iter().zip(field_accessors.iter()) {
            result.push_str(&format!("auto& {} = {}.{}; ", name, temp, accessor));
        }
        result
    }

    /// Expande `std::optional<T>` → struct con bool has_value + T value
    pub fn expand_optional(
        &mut self,
        inner_type: &str,
    ) -> String {
        let name = self.unique_name("optional");
        format!(
            "struct {} {{ bool __has_value; {} __value; {} () : __has_value(false) {{}} {}({} v) : __has_value(true), __value(v) {{}} bool has_value() const {{ return __has_value; }} {}& value() {{ return __value; }} }};",
            name, inner_type, name, name, inner_type, inner_type
        )
    }

    /// Expande `std::variant<A, B>` → tagged union
    pub fn expand_variant(
        &mut self,
        types: &[&str],
    ) -> String {
        let name = self.unique_name("variant");
        let mut fields = String::new();
        for (i, ty) in types.iter().enumerate() {
            fields.push_str(&format!("{} __v{}; ", ty, i));
        }
        format!(
            "struct {} {{ int __tag; union {{ {} }}; }};",
            name, fields
        )
    }

    /// Expande `std::string_view` → `const char*` + `size_t len`
    pub fn expand_string_view(&mut self) -> String {
        let name = self.unique_name("string_view");
        format!(
            "struct {} {{ const char* __data; unsigned long long __len; {} (const char* s, unsigned long long n) : __data(s), __len(n) {{}} const char* data() const {{ return __data; }} unsigned long long size() const {{ return __len; }} }};",
            name, name
        )
    }

    /// Expande fold expressions C++17: `(args + ...)`
    /// Dado los valores concretos, genera la expansion
    pub fn expand_fold_expression(
        &self,
        op: &str,
        values: &[&str],
    ) -> String {
        if values.is_empty() {
            return "0".to_string();
        }
        values.join(&format!(" {} ", op))
    }

    /// Expande `if constexpr` con evaluacion de rama muerta eliminada
    /// (ya existe expand_if_constexpr, esta version es mas completa con scope)
    pub fn expand_if_constexpr_scoped(
        &mut self,
        condition_is_true: bool,
        then_body: &str,
        else_body: Option<&str>,
    ) -> String {
        let scope = self.unique_name("constexpr_scope");
        if condition_is_true {
            format!("/* {}: true branch */ {{ {} }}", scope, then_body)
        } else {
            match else_body {
                Some(body) => format!("/* {}: false branch */ {{ {} }}", scope, body),
                None => format!("/* {}: dead branch eliminated */", scope),
            }
        }
    }

    /// Expande `[[nodiscard]]` — marca retorno que no debe ignorarse
    pub fn expand_nodiscard(&self, func_name: &str) -> String {
        format!("/* [[nodiscard]] {} — return value must be used */", func_name)
    }

    /// Expande `[[maybe_unused]]` — suprime warning de variable no usada
    pub fn expand_maybe_unused(&self, var_name: &str) -> String {
        format!("/* [[maybe_unused]] {} */ (void){};", var_name, var_name)
    }

    /// Expande `[[fallthrough]]` — indica fall-through intencional en switch
    pub fn expand_fallthrough(&self) -> String {
        "/* [[fallthrough]] */".to_string()
    }

    /// Expande nested namespaces C++17: `namespace A::B::C { }`
    /// → `namespace A { namespace B { namespace C { } } }`
    pub fn expand_nested_namespace(
        &self,
        parts: &[&str],
        body: &str,
    ) -> String {
        let mut result = String::new();
        for part in parts {
            result.push_str(&format!("namespace {} {{ ", part));
        }
        result.push_str(body);
        for _ in parts {
            result.push_str(" }");
        }
        result
    }

    /// Expande `constexpr if` con type traits check
    pub fn expand_type_trait_check(
        &self,
        trait_name: &str,
        type_name: &str,
        result: bool,
    ) -> String {
        format!(
            "/* {}::<{}> = {} */",
            trait_name, type_name, result
        )
    }

    /// Expande `inline variable` C++17: variable con linkage inline
    pub fn expand_inline_variable(
        &self,
        var_type: &str,
        var_name: &str,
        init_value: &str,
    ) -> String {
        format!("static {} {} = {};", var_type, var_name, init_value)
    }

    /// Expande `std::any` → void* con type tag
    pub fn expand_any(&mut self) -> String {
        let name = self.unique_name("any");
        format!(
            "struct {} {{ void* __ptr; int __type_id; {} () : __ptr(0), __type_id(0) {{}} }};",
            name, name
        )
    }

    /// Retorna la lista completa de features C++11/14/17 soportadas
    pub fn supported_features() -> Vec<&'static str> {
        vec![
            // C++11
            "lambda",
            "range-for",
            "auto",
            "nullptr",
            "static_assert",
            "enum class",
            "using alias",
            "variadic templates",
            "constexpr functions",
            "move semantics",
            "initializer_list",
            "delegating constructors",
            // C++14
            "generic lambda",
            "[[deprecated]]",
            "binary literals",
            "digit separators",
            "return type deduction",
            "make_unique",
            // C++17
            "structured bindings",
            "if constexpr",
            "std::optional",
            "std::variant",
            "std::string_view",
            "std::any",
            "fold expressions",
            "[[nodiscard]]",
            "[[maybe_unused]]",
            "[[fallthrough]]",
            "nested namespaces",
            "inline variables",
            "constexpr if (scoped)",
            "type traits check",
        ]
    }
}

impl Default for MacroExpander {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lambda_expansion() {
        let mut expander = MacroExpander::new();
        let result = expander.expand_lambda("int x", "return x + 1;");
        assert!(result.contains("__lambda_1__"));
        assert!(result.contains("operator()"));
    }

    #[test]
    fn test_range_for_expansion() {
        let expander = MacroExpander::new();
        let result = expander.expand_range_for("item", "lista", "process(item);");
        assert!(result.contains(".begin()"));
        assert!(result.contains(".end()"));
    }

    #[test]
    fn test_if_constexpr() {
        let expander = MacroExpander::new();
        let result = expander.expand_if_constexpr(true, "branch_true();", Some("branch_false();"));
        assert_eq!(result, "branch_true();");
    }

    #[test]
    fn test_auto_var() {
        let expander = MacroExpander::new();
        let result = expander.expand_auto_var("x", "int", "42");
        assert_eq!(result, "int x = 42;");
    }

    #[test]
    fn test_nullptr() {
        let expander = MacroExpander::new();
        assert_eq!(expander.expand_nullptr(), "((void*)0)");
    }

    #[test]
    fn test_static_assert_pass() {
        let expander = MacroExpander::new();
        assert!(expander.expand_static_assert(true, "size check").is_ok());
    }

    #[test]
    fn test_static_assert_fail() {
        let expander = MacroExpander::new();
        assert!(expander.expand_static_assert(false, "size check").is_err());
    }

    #[test]
    fn test_enum_class() {
        let mut expander = MacroExpander::new();
        let result = expander.expand_enum_class("Color", &["Red", "Green", "Blue"]);
        assert!(result.contains("struct Color"));
        assert!(result.contains("Red, Green, Blue"));
    }

    #[test]
    fn test_using_alias() {
        let expander = MacroExpander::new();
        let result = expander.expand_using_alias("StringVec", "std::vector<std::string>");
        assert_eq!(result, "typedef std::vector<std::string> StringVec;");
    }

    #[test]
    fn test_structured_binding() {
        let mut expander = MacroExpander::new();
        let result = expander.expand_structured_binding(
            &["a", "b"],
            "my_pair",
            &["first", "second"],
        );
        assert!(result.contains("a = "));
        assert!(result.contains("b = "));
        assert!(result.contains(".first"));
        assert!(result.contains(".second"));
    }

    #[test]
    fn test_fold_expression() {
        let expander = MacroExpander::new();
        let result = expander.expand_fold_expression("+", &["1", "2", "3"]);
        assert_eq!(result, "1 + 2 + 3");
    }

    #[test]
    fn test_fold_expression_empty() {
        let expander = MacroExpander::new();
        let result = expander.expand_fold_expression("+", &[]);
        assert_eq!(result, "0");
    }

    #[test]
    fn test_binary_literal() {
        let expander = MacroExpander::new();
        assert_eq!(expander.expand_binary_literal("0b1010").unwrap(), "10");
        assert_eq!(expander.expand_binary_literal("0B11111111").unwrap(), "255");
    }

    #[test]
    fn test_digit_separator() {
        let expander = MacroExpander::new();
        assert_eq!(expander.expand_digit_separator("1'000'000"), "1000000");
    }

    #[test]
    fn test_nested_namespace() {
        let expander = MacroExpander::new();
        let result = expander.expand_nested_namespace(
            &["A", "B", "C"],
            "int x = 1;",
        );
        assert!(result.contains("namespace A"));
        assert!(result.contains("namespace B"));
        assert!(result.contains("namespace C"));
        assert!(result.contains("int x = 1;"));
    }

    #[test]
    fn test_optional() {
        let mut expander = MacroExpander::new();
        let result = expander.expand_optional("int");
        assert!(result.contains("has_value"));
        assert!(result.contains("__value"));
    }

    #[test]
    fn test_variant() {
        let mut expander = MacroExpander::new();
        let result = expander.expand_variant(&["int", "float", "char"]);
        assert!(result.contains("__tag"));
        assert!(result.contains("union"));
    }

    #[test]
    fn test_nodiscard() {
        let expander = MacroExpander::new();
        let result = expander.expand_nodiscard("getValue");
        assert!(result.contains("nodiscard"));
        assert!(result.contains("getValue"));
    }

    #[test]
    fn test_maybe_unused() {
        let expander = MacroExpander::new();
        let result = expander.expand_maybe_unused("temp");
        assert!(result.contains("maybe_unused"));
        assert!(result.contains("(void)temp"));
    }

    #[test]
    fn test_inline_variable() {
        let expander = MacroExpander::new();
        let result = expander.expand_inline_variable("const int", "MAX_SIZE", "1024");
        assert_eq!(result, "static const int MAX_SIZE = 1024;");
    }

    #[test]
    fn test_supported_features_count() {
        let features = MacroExpander::supported_features();
        assert!(features.len() >= 30); // At least 30 C++11/14/17 features
    }

    #[test]
    fn test_make_unique() {
        let expander = MacroExpander::new();
        let result = expander.expand_make_unique("Widget", "10, 20");
        assert_eq!(result, "new Widget(10, 20)");
    }

    #[test]
    fn test_generic_lambda() {
        let mut expander = MacroExpander::new();
        let result = expander.expand_generic_lambda(
            &[("int", "x"), ("double", "y")],
            "return x + y;",
        );
        assert!(result.contains("operator()"));
        assert!(result.contains("int x"));
        assert!(result.contains("double y"));
    }

    #[test]
    fn test_variadic_template() {
        let mut expander = MacroExpander::new();
        let result = expander.expand_variadic_template(
            "print",
            &["int", "float"],
            "/* body */",
        );
        assert!(result.contains("__variadic_print_int_float__"));
    }

    #[test]
    fn test_constexpr_func() {
        let expander = MacroExpander::new();
        let result = expander.expand_constexpr_func("int", "MAX", "100");
        assert!(result.contains("static const int MAX = 100"));
    }

    #[test]
    fn test_string_view() {
        let mut expander = MacroExpander::new();
        let result = expander.expand_string_view();
        assert!(result.contains("__data"));
        assert!(result.contains("__len"));
        assert!(result.contains("data()"));
        assert!(result.contains("size()"));
    }
}
