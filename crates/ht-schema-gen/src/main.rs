use std::{
    collections::BTreeMap,
    env, fs,
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};

const MODELS: &[&str] = &[
    "Task",
    "Project",
    "Section",
    "Comment",
    "Notification",
    "Assignees",
    "Label",
    "TaskLabel",
];

fn blocks(source: &str, kind: &str) -> BTreeMap<String, Vec<String>> {
    let mut result = BTreeMap::new();
    let mut current = None;
    for line in source.lines() {
        let line = line.split("//").next().unwrap().trim();
        if let Some(rest) = line.strip_prefix(&format!("{kind} ")) {
            current = Some((
                rest.split_whitespace().next().unwrap().to_owned(),
                Vec::new(),
            ));
        } else if line == "}" {
            if let Some((name, lines)) = current.take() {
                result.insert(name, lines);
            }
        } else if let Some((_, lines)) = current.as_mut() {
            if !line.is_empty() && !line.starts_with('@') {
                lines.push(line.to_owned());
            }
        }
    }
    result
}

fn generate(source: &str) -> Result<String, Box<dyn std::error::Error>> {
    let models = blocks(source, "model");
    let enums = blocks(source, "enum");
    let mut out = String::from(
        "#![allow(non_snake_case, non_camel_case_types)]\nuse serde::{Deserialize, Serialize};\n",
    );
    let mut ddl = String::new();
    let mut metadata = String::new();
    let mut used_enums = BTreeMap::new();
    for name in MODELS {
        let lines = models
            .get(*name)
            .ok_or_else(|| format!("missing model {name}"))?;
        out.push_str(&format!(
            "#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]\npub struct {name} {{\n"
        ));
        ddl.push_str(&format!("CREATE TABLE IF NOT EXISTS \"{name}\" (\n"));
        let mut fields = Vec::new();
        for line in lines {
            let parts: Vec<_> = line.split_whitespace().collect();
            if parts.len() < 2 || line.contains("@ignore") {
                continue;
            }
            let field = parts[0];
            let base = parts[1].trim_end_matches('?').trim_end_matches("[]");
            let (rust, sql) = match base {
                "Int" | "BigInt" => ("i64", "INTEGER"),
                "Float" => ("f64", "REAL"),
                "Decimal" => ("String", "TEXT"),
                "String" | "DateTime" => ("String", "TEXT"),
                "Boolean" => ("bool", "INTEGER"),
                "Json" => ("serde_json::Value", "TEXT"),
                "Bytes" => ("Vec<u8>", "BLOB"),
                x if enums.contains_key(x) => {
                    used_enums.insert(x.to_owned(), enums[x].clone());
                    (x, "TEXT")
                }
                _ => continue,
            };
            let rust = if parts[1].ends_with("[]") {
                format!("Vec<{rust}>")
            } else if parts[1].ends_with('?') {
                format!("Option<{rust}>")
            } else {
                rust.to_owned()
            };
            let ident = if matches!(field, "type" | "ref" | "move") {
                format!("r#{field}")
            } else {
                field.to_owned()
            };
            out.push_str(&format!("pub {ident}: {rust},\n"));
            let sql = if parts[1].ends_with("[]") {
                "TEXT"
            } else {
                sql
            };
            ddl.push_str(&format!(
                "\"{field}\" {sql}{},\n",
                if line.contains("@id") {
                    " PRIMARY KEY"
                } else {
                    ""
                }
            ));
            fields.push(format!("\"{field}\""));
        }
        out.push_str("}\n");
        ddl.push_str("raw_json TEXT NOT NULL DEFAULT '{}'\n);\n");
        metadata.push_str(&format!("(\"{name}\", &[{}]),\n", fields.join(",")));
    }
    for (name, variants) in used_enums {
        out.push_str(&format!("#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]\npub enum {name} {{\n"));
        for v in variants {
            out.push_str(&format!("{},\n", v.split_whitespace().next().unwrap()));
        }
        out.push_str("}\n");
    }
    out.push_str(&format!("pub const SQLITE_DDL: &str = {ddl:?};\npub const MODEL_COLUMNS: &[(&str, &[&str])] = &[{metadata}];\n"));
    let mut child = Command::new("rustfmt")
        .args(["--edition", "2024", "--emit", "stdout"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    child.stdin.take().unwrap().write_all(out.as_bytes())?;
    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err("rustfmt failed".into());
    }
    Ok(String::from_utf8(output.stdout)?)
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut check = false;
    let mut schema = None;
    let mut output = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../ht-core/src/generated.rs");
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--check" => check = true,
            "--output" => output = args.next().ok_or("--output requires a path")?.into(),
            x if x.starts_with('-') => return Err(format!("unknown option {x}").into()),
            _ if schema.is_none() => schema = Some(arg),
            _ => return Err("only one schema path is accepted".into()),
        }
    }
    let source = fs::read_to_string(
        schema.ok_or("usage: ht-schema-gen <schema.prisma> [--check] [--output path]")?,
    )?;
    let fresh = generate(&source)?;
    if check {
        if fs::read_to_string(&output)? != fresh {
            return Err("generated.rs is stale; run ht-schema-gen <schema.prisma>".into());
        }
    } else {
        fs::write(output, fresh)?;
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_scalar_projection_is_deterministic_and_excludes_relations() {
        let mut schema = String::from("enum Status {\nNormal\nArchive\nDeleted\n}\n");
        for name in MODELS {
            schema.push_str(&format!("model {name} {{\nid Int @id\nname String?\nstatus Status\nseen Int[]\nactivity Json?\nrelation User @relation(fields: [id], references: [id])\nignored String? @ignore\n}}\n"));
        }
        let first = generate(&schema).unwrap();
        assert_eq!(first, generate(&schema).unwrap());
        assert!(first.contains("pub name: Option<String>"));
        assert!(first.contains("pub seen: Vec<i64>"));
        assert!(first.contains("pub activity: Option<serde_json::Value>"));
        assert!(first.contains("pub enum Status"));
        assert!(!first.contains("pub relation:"));
        assert!(!first.contains("pub ignored:"));
        for name in MODELS {
            assert!(first.contains(&format!("pub struct {name}")));
        }
    }

    #[test]
    fn schema_missing_required_model_fails_closed() {
        assert!(generate("model Task {\nid Int @id\n}\n").is_err());
    }
}
