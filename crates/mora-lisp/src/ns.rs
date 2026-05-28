use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::Mutex;

use crate::types::{Symbol, Value};

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
pub struct NamespaceRegistry {
    namespaces: HashMap<String, Arc<Mutex<Namespace>>>,
    current: Arc<Mutex<Namespace>>,
}

impl NamespaceRegistry {
    pub fn new() -> Self {
        let user_ns = Arc::new(Mutex::new(Namespace::new("user")));
        let mut namespaces = HashMap::new();
        namespaces.insert("user".to_string(), user_ns.clone());

        let mut reg = Self {
            namespaces,
            current: user_ns,
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

    pub fn require(&mut self, ns_name: &str, alias: Option<&str>) -> Result<(), String> {
        let ns = self.find_or_create(ns_name);
        let mut current = self.current.lock();
        if let Some(alias_name) = alias {
            current.alias(alias_name, ns_name);
        }
        // Import public vars from the required namespace
        let required = ns.lock();
        for (name, var) in &required.vars {
            if !var.is_private {
                current.refers.insert(name.clone(), var.clone());
            }
        }
        Ok(())
    }

    pub fn refer_all(&mut self, from_ns: &str, to_ns: &str) -> Result<(), String> {
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

            return self
                .namespaces
                .get(&resolved_ns)
                .and_then(|ns| ns.lock().find_var(sym.name.as_str()).map(|v| v.deref()));
        }

        current.find_var(sym.name.as_str()).map(|v| v.deref())
    }
}
