#[cfg(test)]
mod tests {
    use mora_lisp::reader;
    use mora_lisp::types::Value;
    use mora_lisp::MoraLisp;

    #[test]
    fn test_read_numbers() {
        let val = reader::read_str("42").unwrap();
        assert_eq!(val, Value::Int(42));

        let val = reader::read_str("-7").unwrap();
        assert_eq!(val, Value::Int(-7));

        let val = reader::read_str("3.14").unwrap();
        assert_eq!(val, Value::Float(3.14));
    }

    #[test]
    fn test_read_strings() {
        let val = reader::read_str("\"hello\"").unwrap();
        assert_eq!(val, Value::string("hello"));

        let val = reader::read_str("\"hello\\nworld\"").unwrap();
        assert_eq!(val, Value::string("hello\nworld"));
    }

    #[test]
    fn test_read_bools() {
        assert_eq!(reader::read_str("true").unwrap(), Value::Bool(true));
        assert_eq!(reader::read_str("false").unwrap(), Value::Bool(false));
        assert_eq!(reader::read_str("nil").unwrap(), Value::Nil);
    }

    #[test]
    fn test_read_keywords() {
        let val = reader::read_str(":foo").unwrap();
        match val {
            Value::Keyword(k) => assert_eq!(k.name.as_str(), "foo"),
            _ => panic!("expected keyword"),
        }
    }

    #[test]
    fn test_read_symbols() {
        let val = reader::read_str("x").unwrap();
        match val {
            Value::Symbol(s) => assert_eq!(s.name.as_str(), "x"),
            _ => panic!("expected symbol"),
        }
    }

    #[test]
    fn test_read_list() {
        let val = reader::read_str("(1 2 3)").unwrap();
        match val {
            Value::List(v) => {
                assert_eq!(v.len(), 3);
                assert_eq!(v[0], Value::Int(1));
                assert_eq!(v[1], Value::Int(2));
                assert_eq!(v[2], Value::Int(3));
            }
            _ => panic!("expected list"),
        }
    }

    #[test]
    fn test_read_vector() {
        let val = reader::read_str("[1 2 3]").unwrap();
        match val {
            Value::Vector(v) => {
                assert_eq!(v.len(), 3);
            }
            _ => panic!("expected vector"),
        }
    }

    #[test]
    fn test_read_map() {
        let val = reader::read_str("{:a 1 :b 2}").unwrap();
        match val {
            Value::Map(m) => {
                assert_eq!(m.len(), 2);
            }
            _ => panic!("expected map"),
        }
    }

    #[test]
    fn test_read_nested() {
        let val = reader::read_str("(+ 1 (- 3 2))").unwrap();
        match val {
            Value::List(v) => {
                assert_eq!(v.len(), 3);
            }
            _ => panic!("expected list"),
        }
    }

    #[test]
    fn test_read_quote() {
        let val = reader::read_str("'x").unwrap();
        match val {
            Value::List(v) => {
                assert_eq!(v.len(), 2);
            }
            _ => panic!("expected list (quote)"),
        }
    }

    #[test]
    fn test_eval_arithmetic() {
        let mut lisp = MoraLisp::new();
        let result = lisp.eval("(+ 1 2 3)").unwrap();
        assert_eq!(result, Value::Int(6));

        let result = lisp.eval("(- 10 3)").unwrap();
        assert_eq!(result, Value::Int(7));

        let result = lisp.eval("(* 2 3 4)").unwrap();
        assert_eq!(result, Value::Int(24));

        let result = lisp.eval("(/ 10 2)").unwrap();
        assert_eq!(result, Value::Int(5));
    }

    #[test]
    fn test_eval_comparison() {
        let mut lisp = MoraLisp::new();
        assert_eq!(lisp.eval("(= 1 1)").unwrap(), Value::Bool(true));
        assert_eq!(lisp.eval("(= 1 2)").unwrap(), Value::Bool(false));
        assert_eq!(lisp.eval("(< 1 2)").unwrap(), Value::Bool(true));
        assert_eq!(lisp.eval("(> 2 1)").unwrap(), Value::Bool(true));
        assert_eq!(lisp.eval("(<= 1 1)").unwrap(), Value::Bool(true));
        assert_eq!(lisp.eval("(>= 2 1)").unwrap(), Value::Bool(true));
    }

    #[test]
    fn test_eval_logic() {
        let mut lisp = MoraLisp::new();
        assert_eq!(lisp.eval("(not true)").unwrap(), Value::Bool(false));
        assert_eq!(lisp.eval("(not false)").unwrap(), Value::Bool(true));
        assert_eq!(lisp.eval("(not nil)").unwrap(), Value::Bool(true));
    }

    #[test]
    fn test_eval_def() {
        let mut lisp = MoraLisp::new();
        lisp.eval("(def x 42)").unwrap();
        let result = lisp.eval("x").unwrap();
        assert_eq!(result, Value::Int(42));
    }

    #[test]
    fn test_eval_if() {
        let mut lisp = MoraLisp::new();
        assert_eq!(lisp.eval("(if true 1 2)").unwrap(), Value::Int(1));
        assert_eq!(lisp.eval("(if false 1 2)").unwrap(), Value::Int(2));
        assert_eq!(lisp.eval("(if nil 1 2)").unwrap(), Value::Int(2));
        assert_eq!(lisp.eval("(if 0 1 2)").unwrap(), Value::Int(1));
    }

    #[test]
    fn test_eval_do() {
        let mut lisp = MoraLisp::new();
        let result = lisp.eval("(do 1 2 3)").unwrap();
        assert_eq!(result, Value::Int(3));
    }

    #[test]
    fn test_eval_let() {
        let mut lisp = MoraLisp::new();
        let result = lisp.eval("(let [x 10 y 20] (+ x y))").unwrap();
        assert_eq!(result, Value::Int(30));
    }

    #[test]
    fn test_eval_fn() {
        let mut lisp = MoraLisp::new();
        lisp.eval("(def add (fn [a b] (+ a b)))").unwrap();
        let result = lisp.eval("(add 3 4)").unwrap();
        assert_eq!(result, Value::Int(7));
    }

    #[test]
    fn test_eval_defn() {
        let mut lisp = MoraLisp::new();
        lisp.eval("(defn multiply [a b] (* a b))").unwrap();
        let result = lisp.eval("(multiply 5 6)").unwrap();
        assert_eq!(result, Value::Int(30));
    }

    #[test]
    fn test_eval_defn_docstring() {
        let mut lisp = MoraLisp::new();
        lisp.eval("(defn answer \"Return the answer.\" [] 42)")
            .unwrap();
        assert_eq!(lisp.eval("(answer)").unwrap(), Value::Int(42));
    }

    #[test]
    fn test_eval_defn_interactive_marker_is_not_executed() {
        let mut lisp = MoraLisp::new();
        lisp.eval("(defn answer [] (interactive) 42)").unwrap();
        assert_eq!(lisp.eval("(answer)").unwrap(), Value::Int(42));
    }

    #[test]
    fn test_eval_defcommand_defines_function_without_host_registry() {
        let mut lisp = MoraLisp::new();
        lisp.eval("(defcommand meaning [] 42)").unwrap();
        assert_eq!(lisp.eval("(meaning)").unwrap(), Value::Int(42));
    }

    #[test]
    fn test_eval_lambda() {
        let mut lisp = MoraLisp::new();
        let result = lisp.eval("((fn [x] (* x x)) 5)").unwrap();
        assert_eq!(result, Value::Int(25));
    }

    #[test]
    fn test_collections() {
        let mut lisp = MoraLisp::new();
        assert_eq!(lisp.eval("(count [1 2 3])").unwrap(), Value::Int(3));
        assert_eq!(lisp.eval("(count '(1 2))").unwrap(), Value::Int(2));
        assert_eq!(lisp.eval("(first [1 2 3])").unwrap(), Value::Int(1));
        assert_eq!(lisp.eval("(last [1 2 3])").unwrap(), Value::Int(3));
        assert_eq!(
            lisp.eval("(rest [1 2 3])").unwrap(),
            Value::list(vec![Value::Int(2), Value::Int(3)])
        );
    }

    #[test]
    fn test_eval_str() {
        let mut lisp = MoraLisp::new();
        let result = lisp.eval("(str \"hello\" \" \" \"world\")").unwrap();
        assert_eq!(result, Value::string("hello world"));
    }

    #[test]
    fn test_eval_cond() {
        let mut lisp = MoraLisp::new();
        let result = lisp.eval("(cond false 1 true 2 false 3)").unwrap();
        assert_eq!(result, Value::Int(2));
    }

    #[test]
    fn test_eval_when() {
        let mut lisp = MoraLisp::new();
        assert_eq!(lisp.eval("(when true 42)").unwrap(), Value::Int(42));
        assert_eq!(lisp.eval("(when false 42)").unwrap(), Value::Nil);
    }

    #[test]
    fn test_eval_and_or() {
        let mut lisp = MoraLisp::new();
        assert_eq!(lisp.eval("(and true true)").unwrap(), Value::Bool(true));
        assert_eq!(lisp.eval("(and true false)").unwrap(), Value::Bool(false));
        assert_eq!(lisp.eval("(or false true)").unwrap(), Value::Bool(true));
        assert_eq!(lisp.eval("(or false false)").unwrap(), Value::Bool(false));
    }

    #[test]
    fn test_type_predicates() {
        let mut lisp = MoraLisp::new();
        assert_eq!(lisp.eval("(nil? nil)").unwrap(), Value::Bool(true));
        assert_eq!(lisp.eval("(nil? 1)").unwrap(), Value::Bool(false));
        assert_eq!(lisp.eval("(int? 42)").unwrap(), Value::Bool(true));
        assert_eq!(lisp.eval("(string? \"hello\")").unwrap(), Value::Bool(true));
        assert_eq!(lisp.eval("(boolean? true)").unwrap(), Value::Bool(true));
        assert_eq!(lisp.eval("(vector? [1 2])").unwrap(), Value::Bool(true));
        assert_eq!(lisp.eval("(list? '(1 2))").unwrap(), Value::Bool(true));
        assert_eq!(lisp.eval("(map? {:a 1})").unwrap(), Value::Bool(true));
    }

    #[test]
    fn test_atom() {
        let mut lisp = MoraLisp::new();
        lisp.eval("(def a (atom 0))").unwrap();
        assert_eq!(lisp.eval("(deref a)").unwrap(), Value::Int(0));
        lisp.eval("(swap! a (fn [x] (+ x 1)))").unwrap();
        assert_eq!(lisp.eval("(deref a)").unwrap(), Value::Int(1));
        lisp.eval("(reset! a 42)").unwrap();
        assert_eq!(lisp.eval("(deref a)").unwrap(), Value::Int(42));
    }

    #[test]
    fn test_namespace() {
        let mut lisp = MoraLisp::new();
        lisp.eval("(ns my.ns)").unwrap();
        lisp.eval("(def x 10)").unwrap();
        assert_eq!(lisp.eval("x").unwrap(), Value::Int(10));
    }

    #[test]
    fn test_core_namespace_is_qualified_and_referred() {
        let mut lisp = MoraLisp::new();
        assert_eq!(lisp.eval("(mora.core/+ 1 2)").unwrap(), Value::Int(3));
        lisp.eval("(ns coldnew.config)").unwrap();
        assert_eq!(lisp.eval("(+ 2 3)").unwrap(), Value::Int(5));
    }

    #[test]
    fn test_require_alias_resolves_qualified_symbols() {
        let mut lisp = MoraLisp::new();
        lisp.eval("(ns coldnew.alpha)").unwrap();
        lisp.eval("(def value 42)").unwrap();
        lisp.eval("(ns coldnew.beta)").unwrap();
        lisp.eval("(require [coldnew.alpha :as alpha])").unwrap();
        assert_eq!(lisp.eval("alpha/value").unwrap(), Value::Int(42));
    }

    #[test]
    fn test_eval_loop() {
        let mut lisp = MoraLisp::new();
        let result = lisp
            .eval("(loop [i 0 acc 0] (if (= i 5) acc (recur (+ i 1) (+ acc i))))")
            .unwrap();
        assert_eq!(result, Value::Int(10));
    }

    #[test]
    fn test_map_ops() {
        let mut lisp = MoraLisp::new();
        assert_eq!(lisp.eval("(get {:a 1 :b 2} :a)").unwrap(), Value::Int(1));
        assert_eq!(
            lisp.eval("(contains? {:a 1} :a)").unwrap(),
            Value::Bool(true)
        );
        assert_eq!(lisp.eval("(count {:a 1 :b 2})").unwrap(), Value::Int(2));
    }

    #[test]
    fn test_conj() {
        let mut lisp = MoraLisp::new();
        let result = lisp.eval("(conj [1 2] 3)").unwrap();
        match result {
            Value::Vector(v) => {
                assert_eq!(v.len(), 3);
                assert_eq!(v[2], Value::Int(3));
            }
            _ => panic!("expected vector"),
        }
    }

    #[test]
    fn test_concat() {
        let mut lisp = MoraLisp::new();
        let result = lisp.eval("(concat [1 2] [3 4])").unwrap();
        match result {
            Value::List(v) => assert_eq!(v.len(), 4),
            _ => panic!("expected list"),
        }
    }

    #[test]
    fn test_nested_eval() {
        let mut lisp = MoraLisp::new();
        let result = lisp.eval("(+ (+ 1 2) (+ 3 4))").unwrap();
        assert_eq!(result, Value::Int(10));
    }

    #[test]
    fn test_closure() {
        let mut lisp = MoraLisp::new();
        lisp.eval("(defn make-adder [x] (fn [y] (+ x y)))").unwrap();
        lisp.eval("(def add5 (make-adder 5))").unwrap();
        let result = lisp.eval("(add5 10)").unwrap();
        assert_eq!(result, Value::Int(15));
    }

    #[test]
    fn test_thread_first() {
        let mut lisp = MoraLisp::new();

        // Basic: (-> 5 inc) => (inc 5) => 6
        let result = lisp.eval("(-> 5 inc)").unwrap();
        assert_eq!(result, Value::Int(6));

        // Chained: (-> 5 inc inc) => (inc (inc 5)) => 7
        let result = lisp.eval("(-> 5 inc inc)").unwrap();
        assert_eq!(result, Value::Int(7));

        // With multi-arg fn: (-> 5 (+ 3)) => (+ 5 3) => 8
        let result = lisp.eval("(-> 5 (+ 3))").unwrap();
        assert_eq!(result, Value::Int(8));

        // Complex: (-> 5 (+ 3) (* 2)) => (* (+ 5 3) 2) => 16
        let result = lisp.eval("(-> 5 (+ 3) (* 2))").unwrap();
        assert_eq!(result, Value::Int(16));
    }

    #[test]
    fn test_thread_last() {
        let mut lisp = MoraLisp::new();

        // Basic: (->> 5 inc) => (inc 5) => 6
        let result = lisp.eval("(->> 5 inc)").unwrap();
        assert_eq!(result, Value::Int(6));

        // With multi-arg fn: (->> 5 (+ 3)) => (+ 3 5) => 8
        let result = lisp.eval("(->> 5 (+ 3))").unwrap();
        assert_eq!(result, Value::Int(8));

        // Complex: (->> 5 (+ 3) (* 2)) => (* 2 (+ 3 5)) => 16
        let result = lisp.eval("(->> 5 (+ 3) (* 2))").unwrap();
        assert_eq!(result, Value::Int(16));
    }

    #[test]
    fn test_thread_first_single_value() {
        let mut lisp = MoraLisp::new();
        let result = lisp.eval("(-> 42)").unwrap();
        assert_eq!(result, Value::Int(42));
    }

    #[test]
    fn test_thread_last_single_value() {
        let mut lisp = MoraLisp::new();
        let result = lisp.eval("(->> 42)").unwrap();
        assert_eq!(result, Value::Int(42));
    }

    #[test]
    fn test_thread_first_with_custom_fn() {
        let mut lisp = MoraLisp::new();
        lisp.eval("(defn double [x] (* x 2))").unwrap();
        lisp.eval("(defn add1 [x] (+ x 1))").unwrap();

        // (-> 3 double add1) => (add1 (double 3)) => 7
        let result = lisp.eval("(-> 3 double add1)").unwrap();
        assert_eq!(result, Value::Int(7));
    }

    #[test]
    fn test_thread_last_with_custom_fn() {
        let mut lisp = MoraLisp::new();
        lisp.eval("(defn wrap [x] (vector x))").unwrap();

        // (->> 42 wrap) => (wrap 42) => [42]
        let result = lisp.eval("(->> 42 wrap)").unwrap();
        match result {
            Value::Vector(v) => assert_eq!(v.len(), 1),
            _ => panic!("expected vector"),
        }
    }
}
