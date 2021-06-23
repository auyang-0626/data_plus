use std::u64;

use chrono::{NaiveDateTime};
use chrono::{ DateTime, Utc};
use mysql::prelude::*;
use mysql::*;

#[macro_use]
extern crate easy_db_derive;

#[derive(Debug)]
#[derive(Entity)]
#[TableName(data_source)]
#[Query(get_by_name)]
#[Query(get_by_created_by)]
struct DataSource {
    id: Option<u64>,
    name: String,
    source_type: String,
    config: Option<String>,
    created_by: String,
    updated_by: String,
    gmt_create: DateTime<Utc>,
    gmt_modify: DateTime<Utc>,
}

impl DataSource {

    // fn insert2(pool: Pool, entity: &DataSource) -> u64 {
    //     let mut conn: mysql::PooledConn = pool.get_conn().unwrap();
    //     let mut tx = conn.start_transaction(TxOpts::default()).unwrap();
    //     tx.exec_drop("INSERT INTO data_source (name, source_type, config, created_by, updated_by, gmt_create, gmt_modify) VALUES (:name, :source_type, :config, :created_by, :updated_by, :gmt_create, :gmt_modify)", 
    //                 params! {
    //                     "name"=>entity.name.to_string(),
    //                      "source_type"=>entity.source_type.to_string(),
    //                       "config"=>{
    //                           match &entity.config {
    //                               Some(v) => v.to_string(),
    //                               None => String::from(""),
    //                           }
    //                       },
    //                        "created_by"=>entity.created_by.to_string(),
    //                         "updated_by"=>entity.updated_by.to_string(), 
    //                         "gmt_create"=>entity.gmt_create.format(&"%Y-%m-%d %H:%M:%S.%3f").to_string(),
    //                          "gmt_modify"=>entity.gmt_modify.format(&"%Y-%m-%d %H:%M:%S.%3f").to_string(),
    //                 }).unwrap();

    //     let id = tx.last_insert_id().unwrap();
    //     tx.commit().unwrap();
    //     id
    // }
        // fn get_by_id2(pool: Pool,id:u64, params:Params) ->Option<DataSource> {

        //     let mut conn: mysql::PooledConn = pool.get_conn().unwrap();
        //     let stmt = conn.prep("SELECT id, name, source_type, config, created_by, updated_by, gmt_create, gmt_modify FROM data_source where id = :id").unwrap();
        //     let row:Option<Row> = conn.exec_first(stmt, params).unwrap();

        //     match row {
        //         Some(r) => {
        //             Some(DataSource{
        //                 id:Some(r.get(0).unwrap()),
        //                 name:r.get(1).unwrap(),
        //                 source_type:r.get(2).unwrap(),
        //                 config:r.get(3),
        //                 created_by:r.get(4).unwrap(),
        //                 updated_by:r.get(5).unwrap(),
        //                 gmt_create:  DateTime::<Utc>::from_utc(from_value::<NaiveDateTime>(r.get(6).unwrap()), Utc),
        //                 gmt_modify:  DateTime::<Utc>::from_utc(from_value::<NaiveDateTime>(r.get(7).unwrap()), Utc),
        //             })  
        //         },
        //         None => None,
        //     }
        // }
      
}

fn main() {
    let url = "mysql://root:123456@localhost:3306/data_pus";
    let pool = Pool::new(url).unwrap();
    // test_insert(pool);
   println!("{:?}",DataSource::get_by_created_by(pool, params! {"created_by" => "test"}));
}

fn test_insert(pool:Pool){
    let source = DataSource {
        id: None,
        name: String::from("test17"),
        source_type: String::from("test"),
        config: None,
        created_by: String::from("test"),
        updated_by: String::from("test"),
        gmt_create: Utc::now(),
        gmt_modify: Utc::now(),
    };
    println!(
        "{},id:{}",
        DataSource::table_name(),
        DataSource::insert(pool, &source)
    );
}