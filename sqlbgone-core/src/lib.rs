use std::collections::HashMap;

use sqlparser::{
    ast::{Expr, FunctionArg, FunctionArgExpr, Query, TableWithJoins},
    dialect::GenericDialect,
    parser::Parser,
};

pub type DBDefinition = HashMap<String, HashMap<String, DataType>>;

#[derive(Debug, Clone)]
pub enum DataType {
    Null,
    Integer,
    Real,
    Text,
    Blob,
}

fn to_datatype(e: sqlparser::ast::ColumnDef) -> DataType {
    match e.data_type {
        sqlparser::ast::DataType::Blob(_) => DataType::Blob,
        sqlparser::ast::DataType::Integer(_) => DataType::Integer,
        sqlparser::ast::DataType::Real => DataType::Real,
        sqlparser::ast::DataType::Text => DataType::Text,
        _ => todo!(),
    }
}

// TODO: don't hold value, just ref
#[derive(Debug, Clone)]
enum QueryEnv {
    Name(String),
    Table(TableWithJoins),
}

// impl QueryEnv {
//     fn get_type(&self, i: Ident, def: &DBDefinition) -> Option<DataType> {
//         Some(match self {
//             QueryEnv::Name(name) => def.get(name)?.get(&i.value)?.clone(),
//             QueryEnv::Tables(tables) => tables.iter(),
//         })
//     }
// }

fn check_for_placeholer(e: &Expr) -> bool {
    match e {
        Expr::Value(e) => match e {
            sqlparser::ast::Value::Placeholder(_) => true,
            _ => false,
        },
        _ => false,
    }
}

fn process_expr(e: &Expr, env: &QueryEnv, def: &DBDefinition) -> Option<(Vec<DataType>, DataType)> {
    Some(match e {
        Expr::Identifier(e) => (
            vec![],
            match env {
                QueryEnv::Name(n) => def.get(n)?.get(&e.value)?.clone(),
                QueryEnv::Table(table) => table
                    .joins
                    .iter()
                    .map(|t| &t.relation)
                    .chain([&table.relation])
                    .find_map(|table| match table {
                        sqlparser::ast::TableFactor::Table { name, .. } => {
                            def.get(&name.0.get(0)?.value)?.get(&e.value)
                        }
                        _ => todo!(),
                    })
                    .expect("no table?")
                    .clone(),
            },
        ),
        Expr::CompoundIdentifier(e) => (
            vec![],
            def.get(&e.get(0)?.value)?.get(&e.get(1)?.value)?.clone(),
        ),
        Expr::Subquery(query) => {
            let (t_in, t_out) = query_types(def, query)?;
            (t_in, t_out.into_iter().next()?)
        }
        Expr::BinaryOp { left, right, .. } => {
            let mut in_types = vec![];

            if check_for_placeholer(&right) {
                in_types.push(process_expr(&left, env, def)?.1);
            }
            if check_for_placeholer(&left) {
                in_types.push(process_expr(&right, env, def)?.1);
            }
            in_types.extend(process_expr(&left, env, def)?.0);
            in_types.extend(process_expr(&right, env, def)?.0);

            (in_types, DataType::Integer) // TODO: handle syntetic types like Bool
        }
        Expr::Value(v) => match v {
            sqlparser::ast::Value::Placeholder(_) => (vec![], DataType::Null),
            _ => todo!(),
        },
        Expr::Function(f) => {
            if let FunctionArg::Unnamed(fe) = f.args.get(0)? {
                if let FunctionArgExpr::Expr(e) = fe {
                    match f.name.0.get(0)?.value.as_str() {
                        "MAX" | "MIN" => process_expr(e, env, def)?,
                        _ => todo!(),
                    }
                } else {
                    todo!()
                }
            } else {
                todo!()
            }
        }
        _ => todo!(),
    })
}

pub fn get_definition(sql: &str) -> Option<DBDefinition> {
    let def = Parser::parse_sql(&GenericDialect {}, sql).unwrap();
    return Some(
        def.into_iter()
            .filter_map(|e| match e {
                sqlparser::ast::Statement::CreateTable { name, columns, .. } => Some((
                    name.0.get(0).unwrap().to_string(),
                    columns
                        .into_iter()
                        .map(|e| (e.name.to_string(), to_datatype(e)))
                        .collect(),
                )),
                _ => None,
            })
            .collect(),
    );
}

fn query_types(def: &DBDefinition, query: &Query) -> Option<(Vec<DataType>, Vec<DataType>)> {
    let mut in_types = vec![];
    let mut out_types = vec![];

    match *query.body.clone() {
        sqlparser::ast::SetExpr::Select(s) => {
            let env = QueryEnv::Table(s.from.get(0).expect("no table?").clone());
            for pro in s.projection {
                match pro {
                    sqlparser::ast::SelectItem::UnnamedExpr(expr) => {
                        out_types.push(process_expr(&expr, &env, def)?.1);
                    }
                    sqlparser::ast::SelectItem::ExprWithAlias { .. } => todo!(),
                    sqlparser::ast::SelectItem::QualifiedWildcard(_, _) => todo!(),
                    sqlparser::ast::SelectItem::Wildcard(_) => todo!(),
                }
            }
            if let Some(selection) = s.selection {
                in_types.extend(process_expr(&selection, &env, def)?.0);
            }
        }
        _ => todo!(),
    }
    return Some((in_types, out_types));
}

pub fn get_query(def: &DBDefinition, query: &str) -> Option<(Vec<DataType>, Vec<DataType>)> {
    let q_def = Parser::parse_sql(&GenericDialect {}, query).unwrap();

    let mut in_types = vec![];
    let mut out_types = vec![];

    match q_def.get(0).unwrap() {
        sqlparser::ast::Statement::Query(q) => {
            let types = query_types(def, q);
            if let Some((i, o)) = types {
                in_types.extend(i);
                out_types.extend(o);
            }
        }
        sqlparser::ast::Statement::Insert {
            table_name,
            columns,
            source,
            ..
        } => {
            let env = QueryEnv::Name(table_name.0.get(0)?.value.clone());

            if let sqlparser::ast::SetExpr::Values(v) = *source.body.clone() {
                for (col, val) in columns.iter().zip(v.rows[0].iter()) {
                    if check_for_placeholer(val) {
                        in_types.push(
                            def.get(&table_name.0.get(0).unwrap().value)
                                .expect("no table?")
                                .get(&col.value)
                                .expect("no column?")
                                .clone(),
                        );
                    } else {
                        in_types.extend(process_expr(val, &env, def).expect("error processing").0)
                    }
                }
            }
        }
        _ => todo!(),
    }
    Some((in_types, out_types))
}
