extern crate proc_macro;

use syn::Expr;
use syn::ItemImpl;

use proc_macro::TokenStream;
use quote::*;
use syn::{parse_macro_input, DeriveInput};

#[derive(Debug)]
enum FieldType {
    String,
    DateTime,
    Date,
    Number,
}

fn discern_field_type(str: &String) -> (bool, FieldType) {
    let (has_option, type_str) = match str.starts_with("Option <") {
        true => (true, str[8..str.len() - 1].trim()),
        _ => (false, &str[..]),
    };

    if type_str.eq_ignore_ascii_case("String") {
        return (has_option, FieldType::String);
    } else if type_str.starts_with("i") || type_str.starts_with("f") || type_str.starts_with("u") {
        return (has_option, FieldType::Number);
    } else if type_str.contains("DateTime") {
        return (has_option, FieldType::DateTime);
    } else if type_str.starts_with("Date") {
        return (has_option, FieldType::Date);
    } else {
        panic!("invild field type {}", type_str)
    }
}

#[derive(Debug)]
struct FieldDefine {
    name: String,
    column_name: String,
    field_type: FieldType,
    nullable: bool,
}

#[proc_macro_derive(Entity, attributes(TableName, Query))]
pub fn derive_entity(input: TokenStream) -> TokenStream {
    let derive_input = parse_macro_input!(input as DeriveInput);
    // 实体类的名称
    let entity_name = derive_input.ident.to_token_stream();
    // 对应的表名
    let mut table_name = None;
    // 字段列表
    let mut field_vec = Vec::new();
    // 通过注解生成的查询函数
    let mut query_fn_vec = vec![String::from("get_by_id")];

    // 解析结构体的字段
    match derive_input.data {
        syn::Data::Struct(ref data_struct) => match data_struct.fields {
            // field: (0) a: String
            // field: (1) b: Vec < u8 >
            // field: (2) c: T
            syn::Fields::Named(ref fields_named) => {
                for (_, field) in fields_named.named.iter().enumerate() {
                    let name = field.ident.to_token_stream().to_string();
                    let column_name = field.ident.to_token_stream().to_string();
                    let type_str = field.ty.to_token_stream().to_string();
                    let (nullable, field_type) = discern_field_type(&type_str);

                    field_vec.push(FieldDefine {
                        name: name,
                        column_name: column_name,
                        field_type: field_type,
                        nullable: nullable,
                    });
                }
            }
            _ => panic!("must is Named Field !"),
        },
        _ => panic!("must is Struct!"),
    }

    // 解析结构体上的注解
    derive_input.attrs.iter().for_each(|x| {
        let attr = x.path.to_token_stream().to_string();
        let value = x.tokens.to_string();

        if !(value.starts_with("(") && value.ends_with(")")) {
            panic!("Invalid value");
        }
        let rvalue = value[1..value.len() - 1].to_string();

        if attr.eq_ignore_ascii_case("TableName") {
            table_name = Some(rvalue);
        } else if attr.eq_ignore_ascii_case("Query") {
            if !query_fn_vec.contains(&rvalue) {
                query_fn_vec.push(rvalue);
            }
        }
    });
    if table_name.is_none() {
        panic!("miss TableName,eg. #[TableName(t_user)]");
    }
    let table_name = table_name.unwrap();

    // 插入函数
    let (insert_sql, insert_param) = build_insert(&table_name, &field_vec);

    // 查询函数
    let fn_str: String = query_fn_vec
        .iter()
        .map(|x| build_query(&entity_name.to_string(), &table_name, &field_vec, x))
        .collect();

    let fn_items: ItemImpl =
        syn::parse_str(format!("impl DataSource{{ {} }}", fn_str).as_str()).unwrap();

    // ...
    let proc_macro2_token_stream = quote! {
        impl #entity_name {
            fn table_name() -> &'static str {
                #table_name
            }

            fn insert(pool:Pool, entity:&#entity_name) -> u64 {
                let mut conn: mysql::PooledConn = pool.get_conn().unwrap();
                    let mut tx = conn.start_transaction(TxOpts::default()).unwrap();
                    tx.exec_drop(#insert_sql, #insert_param).unwrap();

                let id = tx.last_insert_id().unwrap();
                tx.commit().unwrap();
                id
            }

        }
        #fn_items
    };
    TokenStream::from(proc_macro2_token_stream)
}

// 根据函数名称，构建查询函数
fn build_query(
    entity_name: &String,
    table_name: &String,
    field_vec: &Vec<FieldDefine>,
    fn_name: &String,
) -> String {
    // 分析查询是 get 还是 find，以及根据哪些字段查询
    let (one_result, filters): (bool, Vec<&str>) = {
        if fn_name.starts_with("get_by_") {
            (true, fn_name[7..].split("and").collect())
        } else if fn_name.starts_with("find_by_") {
            (false, fn_name[8..].split("and").collect())
        } else {
            panic!("Invalid fn {}!", fn_name);
        }
    };

    // 拼接查询sql
    let sql = format!(
        "SELECT {} FROM {} where {}",
        field_vec
            .iter()
            .map(|x| x.column_name.to_string())
            .collect::<Vec<String>>()
            .join(", "),
        table_name,
        filters
            .iter()
            .map(|x| format!("{} = :{}", x, x))
            .collect::<Vec<String>>()
            .join(" and ")
    );

    // 拼接 从 row返回值中解析为对应的对象字段的操作
    let mut field_values = String::from("");
    for (i, x) in field_vec.iter().enumerate() {
        //if x.field_type.
        let tmp = match x.field_type {
            FieldType::DateTime => format!(
                "DateTime::<Utc>::from_utc(from_value::<NaiveDateTime>(r.get({}).unwrap()), Utc)",
                i
            ),
            _ => format!("r.get({}).unwrap()", i),
        };
        if x.nullable {
            field_values += &format!("{}:Some({}),", x.column_name.to_string(), tmp);
        } else {
            field_values += &format!("{}:{},", x.column_name.to_string(), tmp);
        }
    }
    format!(
        "fn {}(pool: Pool, params:Params) -> Option<{}> {{
        let mut conn: mysql::PooledConn = pool.get_conn().unwrap();
        let stmt = conn.prep(\" {} \").unwrap();
        let row:Option<Row> = conn.exec_first(stmt, params).unwrap();
        match row {{
            Some(r) => {{
                Some({}{{
                    {}
                }})  
            }},
            None => None,
        }}
     }}
    ",
        fn_name, entity_name, sql, entity_name, field_values
    )
}

// 生成插入函数
fn build_insert(table_name: &String, field_vec: &Vec<FieldDefine>) -> (String, Expr) {
    // 拼接插入sql
    let insert_sql = format!(
        "INSERT INTO {} ({}) VALUES ({})",
        table_name,
        field_vec
            .iter()
            .map(|x| x.column_name.to_string())
            .filter(|x| !x.eq("id"))
            .collect::<Vec<String>>()
            .join(", "),
        field_vec
            .iter()
            .map(|x| x.column_name.to_string())
            .filter(|x| !x.eq("id"))
            .map(|x| String::from(":") + &x)
            .collect::<Vec<String>>()
            .join(", "),
    );
    // 拼接插入的param!表达式
    let param_str = field_vec.iter().filter(|x| !x.column_name.eq("id"))
        .map(|x|{
            // format!("\"{}\"=>self.{}.to_string()",x,x)

            let dft = "%Y-%m-%d %H:%M:%S.%3f";
            let dft2 = "%Y-%m-%d";
            let value_str = match x.field_type{
                FieldType::String => {
                    if x.nullable {
                        format!("match &entity.{} {{Some(v) => v.to_string(), None => String::from(\"\"), }}",x.name)
                    } else {
                        format!("entity.{}.to_string()",x.name)
                    }
                },
                FieldType::Number => {
                    if x.nullable {
                        format!("match &entity.{} {{Some(v) => v, None => String::from(\"null\"), }}",x.name)
                    } else {
                        format!("entity.{}",x.name)
                    }
                },
                FieldType::DateTime => {
                    if x.nullable {
                        format!("match &entity.{} {{Some(v) => v.format(&\"{}\").to_string(), None => String::from(\"null\"), }}",x.name,dft)
                    } else {
                        format!("entity.{}.format(&\"{}\").to_string()",x.name,dft)
                    }
                },
                FieldType::Date => {
                    if x.nullable {
                        format!("match &entity.{} {{Some(v) => v.format(&\"{}\").to_string(), None => String::from(\"null\"), }}",x.name,dft2)
                    } else {
                        format!("entity.{}.format(&\"{}\").to_string()",x.name,dft2)
                    }
                },
            };
            format!("\"{}\"=>{}",x.name,value_str)
        })
        .collect::<Vec<String>>().join(", ");

    // 把param!字符串转为表达式
    let t: Expr = syn::parse_str(&format!("params! {{{}}}", param_str)).unwrap();

    (insert_sql, t)
}
