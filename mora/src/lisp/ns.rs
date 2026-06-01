use crate::lisp::types::{Symbol, Value};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::OnceLock;
/// Global documentation registry for native functions.
/// Maps fully-qualified names (e.g. "mora.buffer/buffer-name") to doc strings.
static DOC_REGISTRY: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
fn doc_registry() -> &'static Mutex<HashMap<String, String>> {
    DOC_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}
/// Register a doc string for a fully-qualified function name.
pub fn register_doc(ns_name: &str, fn_name: &str, doc: &str) {
    let qualified = format!("{}/{}", ns_name, fn_name);
    doc_registry().lock().insert(qualified, doc.to_string());
    // Also register unqualified
    doc_registry()
        .lock()
        .insert(fn_name.to_string(), doc.to_string());
}
/// Look up a doc string by name (qualified or unqualified).
pub fn lookup_doc(name: &str) -> Option<String> {
    doc_registry().lock().get(name).cloned()
}

#[derive(Debug, Clone)]
pub struct Var {
    pub name: Symbol,
    pub value: Arc<Mutex<Value>>,
    pub is_dynamic: bool,
    pub is_private: bool,
    pub is_macro: bool,
    pub meta: Option<Arc<Vec<(Value, Value)>>>,
}

impl Var {
    pub fn new(name: Symbol, value: Value) -> Self {
        Self {
            name,
            value: Arc::new(Mutex::new(value)),
            is_dynamic: false,
            is_private: false,
            is_macro: false,
            meta: None,
        }
    }

    pub fn dynamic(mut self) -> Self {
        self.is_dynamic = true;
        self
    }
    pub fn private(mut self) -> Self {
        self.is_private = true;
        self
    }
    pub fn with_doc(mut self, doc: &str) -> Self {
        let meta = self.meta.get_or_insert_with(|| Arc::new(Vec::new()));
        let mut meta = (**meta).clone();
        meta.push((Value::keyword("doc"), Value::string(doc)));
        self.meta = Some(Arc::new(meta));
        self
    }
    pub fn doc(&self) -> Option<String> {
        self.meta.as_ref().and_then(|meta| {
            meta.iter().find_map(|(k, v)| {
                if k == &Value::keyword("doc") {
                    match v {
                        Value::String(s) => Some(s.to_string()),
                        _ => None,
                    }
                } else {
                    None
                }
            })
        })
    }
    pub fn set(&self, value: Value) {
        *self.value.lock() = value;
    }
    pub fn deref(&self) -> Value {
        self.value.lock().clone()
    }
}
#[derive(Debug, Clone)]
pub struct Namespace {
    pub name: String,
    pub vars: HashMap<String, Var>,
    pub aliases: HashMap<String, String>,
    pub imports: HashMap<String, String>,
    pub refers: HashMap<String, Var>,
}

impl Namespace {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            vars: HashMap::new(),
            aliases: HashMap::new(),
            imports: HashMap::new(),
            refers: HashMap::new(),
        }
    }

    pub fn intern(&mut self, name: &str, value: Value) -> Var {
        let sym = Symbol {
            ns: Some(Arc::new(self.name.clone())),
            name: Arc::new(name.to_string()),
        };
        let var = Var::new(sym, value);
        self.vars.insert(name.to_string(), var.clone());
        var
    }

    pub fn intern_private(&mut self, name: &str, value: Value) -> Var {
        let var = self.intern(name, value).private();
        self.vars.insert(name.to_string(), var.clone());
        var
    }
    pub fn intern_with_doc(&mut self, name: &str, value: Value, doc: &str) -> Var {
        let sym = Symbol {
            ns: Some(Arc::new(self.name.clone())),
            name: Arc::new(name.to_string()),
        };
        let var = Var::new(sym, value).with_doc(doc);
        self.vars.insert(name.to_string(), var.clone());
        register_doc(&self.name, name, doc);
        var
    }
    pub fn intern_private_with_doc(&mut self, name: &str, value: Value, doc: &str) -> Var {
        let var = self.intern_with_doc(name, value, doc).private();
        self.vars.insert(name.to_string(), var.clone());
        var
    }
    pub fn find_var(&self, name: &str) -> Option<&Var> {
        self.vars.get(name).or_else(|| self.refers.get(name))
    }
    pub fn resolve(&self, sym: &Symbol) -> Option<Value> {
        if let Some(ns_name) = &sym.ns {
            // Qualified symbol: check aliases first
            let resolved_ns = self
                .aliases
                .get(ns_name.as_str())
                .map(|s| s.as_str())
                .unwrap_or(ns_name.as_str());
            if resolved_ns == self.name {
                return self.vars.get(sym.name.as_str()).map(|v| v.deref());
            }
            // Can't resolve external namespaces without a registry
            None
        } else {
            // Unqualified: check locals, then refers
            self.vars
                .get(sym.name.as_str())
                .or_else(|| self.refers.get(sym.name.as_str()))
                .map(|v| v.deref())
        }
    }

    pub fn alias(&mut self, alias: &str, ns_name: &str) {
        self.aliases.insert(alias.to_string(), ns_name.to_string());
    }

    pub fn refer(&mut self, name: &str, var: Var) {
        self.refers.insert(name.to_string(), var);
    }
}

/// Global namespace registry
#[derive(Clone)]
pub struct NamespaceRegistry {
    namespaces: HashMap<String, Arc<Mutex<Namespace>>>,
    current: Arc<Mutex<Namespace>>,
    loaded: std::collections::HashSet<String>,
}

impl NamespaceRegistry {
    pub fn new() -> Self {
        let user_ns = Arc::new(Mutex::new(Namespace::new("user")));
        let mut namespaces = HashMap::new();
        namespaces.insert("user".to_string(), user_ns.clone());

        let mut reg = Self {
            namespaces,
            current: user_ns,
            loaded: std::collections::HashSet::new(),
        };

        // Create core namespace with builtins
        reg.create_core_ns();
        reg
    }

    fn create_core_ns(&mut self) {
        let core_ns = Arc::new(Mutex::new(Namespace::new("mora.core")));
        self.namespaces.insert("mora.core".to_string(), core_ns);
    }

    pub fn current(&self) -> Arc<Mutex<Namespace>> {
        self.current.clone()
    }

    pub fn current_name(&self) -> String {
        self.current.lock().name.clone()
    }

    pub fn set_current(&mut self, name: &str) -> Result<(), String> {
        let ns = self
            .namespaces
            .get(name)
            .ok_or_else(|| format!("namespace not found: {}", name))?;
        self.current = ns.clone();
        Ok(())
    }

    pub fn find_or_create(&mut self, name: &str) -> Arc<Mutex<Namespace>> {
        self.namespaces
            .entry(name.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(Namespace::new(name))))
            .clone()
    }

    pub fn find(&self, name: &str) -> Option<Arc<Mutex<Namespace>>> {
        self.namespaces.get(name).cloned()
    }

    pub fn is_loaded(&self, name: &str) -> bool {
        self.loaded.contains(name)
    }

    pub fn mark_loaded(&mut self, name: &str) {
        self.loaded.insert(name.to_string());
    }

    pub fn require(&mut self, ns_name: &str, alias: Option<&str>) -> Result<(), String> {
        let ns = self.find_or_create(ns_name);
        let mut current = self.current.lock();
        if let Some(alias_name) = alias {
            current.alias(alias_name, ns_name);
        }
        if current.name == ns_name {
            return Ok(());
        }
        drop(current);

        // Import public vars from the required namespace
        let public_vars = ns
            .lock()
            .vars
            .iter()
            .filter(|(_, var)| !var.is_private)
            .map(|(name, var)| (name.clone(), var.clone()))
            .collect::<Vec<_>>();
        let mut current = self.current.lock();
        current.refers.extend(public_vars);
        Ok(())
    }

    pub fn refer_all(&mut self, from_ns: &str, to_ns: &str) -> Result<(), String> {
        if from_ns == to_ns {
            return Ok(());
        }

        let source = self
            .namespaces
            .get(from_ns)
            .ok_or_else(|| format!("namespace not found: {}", from_ns))?
            .clone();
        let target = self
            .namespaces
            .get(to_ns)
            .ok_or_else(|| format!("namespace not found: {}", to_ns))?
            .clone();

        let source = source.lock();
        let mut target = target.lock();
        for (name, var) in &source.vars {
            if !var.is_private {
                target.refer(name, var.clone());
            }
        }
        Ok(())
    }

    pub fn resolve_symbol(&self, sym: &Symbol) -> Option<Value> {
        let current = self.current.lock();
        if let Some(ns_name) = &sym.ns {
            let resolved_ns = current
                .aliases
                .get(ns_name.as_str())
                .map(|s| s.as_str())
                .unwrap_or(ns_name.as_str())
                .to_string();
            drop(current);

            // Try resolving from the target namespace
            if let Some(ns) = self.namespaces.get(&resolved_ns) {
                return ns.lock().find_var(sym.name.as_str()).map(|v| v.deref());
            }

            // Fallback: check current namespace's vars with qualified name
            let current = self.current.lock();
            let qualified = format!("{}/{}", ns_name, sym.name);
            return current.vars.get(&qualified).map(|v| v.deref());
        }

        current.find_var(sym.name.as_str()).map(|v| v.deref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refer_all_same_namespace_is_noop() {
        let mut registry = NamespaceRegistry::new();

        registry.refer_all("mora.core", "mora.core").unwrap();
    }

    #[test]
    fn require_current_namespace_is_noop_but_records_alias() {
        let mut registry = NamespaceRegistry::new();
        registry.set_current("mora.core").unwrap();

        registry.require("mora.core", Some("core")).unwrap();

        let current = registry.current();
        let current = current.lock();
        assert_eq!(
            current.aliases.get("core").map(String::as_str),
            Some("mora.core")
        );
    }
}
