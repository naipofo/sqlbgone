use std::collections::HashMap;

use sqlparser::{
    ast::{Expr, TableWithJoins},
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

fn check_for_placeholer(e: Expr) -> bool {
    match e {
        Expr::Value(e) => match e {
            sqlparser::ast::Value::Placeholder(_) => true,
            _ => false,
        },
        _ => false,
    }
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

pub fn get_query(def: &DBDefinition, query: &str) -> Option<(Vec<DataType>, Vec<DataType>)> {
    let q_def = Parser::parse_sql(&GenericDialect {}, query).unwrap();

    let mut in_types = vec![];
    let mut out_types = vec![];

    match q_def.get(0).unwrap() {
        sqlparser::ast::Statement::Query(e) => match *e.body.clone() {
            sqlparser::ast::SetExpr::Select(s) => {
                println!("\n\n\n{:#?}, ", (&s.from, &s.projection, &s.selection));
                fn find_type(
                    iden: &str,
                    table: &TableWithJoins,
                    def: DBDefinition,
                ) -> Option<DataType> {
                    Some(match &table.relation {
                        sqlparser::ast::TableFactor::Table { name, .. } => {
                            let name = name.0.get(0)?.to_string();
                            def.get(&name)?.get(iden)?.clone()
                        }
                        _ => todo!(),
                    })
                }
                for pro in s.projection {
                    match pro {
                        sqlparser::ast::SelectItem::UnnamedExpr(expr) => match expr {
                            sqlparser::ast::Expr::Identifier(expr_iden) => {
                                out_types.push(
                                    find_type(
                                        &expr_iden.to_string(),
                                        s.from.get(0).expect("no table?"),
                                        def.clone(),
                                    )
                                    .expect("type not found"),
                                );
                            }
                            _ => todo!(),
                        },
                        sqlparser::ast::SelectItem::ExprWithAlias { expr, alias } => {
                            todo!()
                        }
                        sqlparser::ast::SelectItem::QualifiedWildcard(_, _) => todo!(),
                        sqlparser::ast::SelectItem::Wildcard(_) => todo!(),
                    }
                }

                if let Some(selection) = s.selection {
                    match selection {
                        sqlparser::ast::Expr::BinaryOp { left, op, right } => {
                            if let Expr::Identifier(iden) = *left {
                                if check_for_placeholer(*right) {
                                    in_types.push(
                                        find_type(
                                            &iden.to_string(),
                                            s.from.get(0).expect("no table?"),
                                            def.clone(),
                                        )
                                        .expect("type not found"),
                                    );
                                }
                            } else if let Expr::Identifier(iden) = *right {
                                if check_for_placeholer(*left) {
                                    in_types.push(
                                        find_type(
                                            &iden.to_string(),
                                            s.from.get(0).expect("no table?"),
                                            def.clone(),
                                        )
                                        .expect("type not found"),
                                    );
                                }
                            }
                        }
                        _ => todo!(),
                    }
                }
            }
            _ => todo!(),
        },
        _ => todo!(),
    }
    Some((in_types, out_types))
}
