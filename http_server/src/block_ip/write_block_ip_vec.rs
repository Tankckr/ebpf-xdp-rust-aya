use crate::{
    block_ip::models::{BlockedIp, WriteIp},
    errors::MyError,
    kill_restart::kill_and_restart::kill_and_restart,
    AppState,
};
use actix_web::{web, HttpResponse};
use regex::Regex;
use std::sync::Arc;

pub async fn write_block_ip_vec(
    state: web::Data<Arc<AppState>>,
    web::Json(write_ip): web::Json<Vec<WriteIp>>,
) -> Result<HttpResponse, MyError> {
    let db_pool = &state.db_pool;

    // 判断传入的IP数组是否为空
    if write_ip.is_empty() {
        return Ok(HttpResponse::Ok().json("传入的IP数量为零,请填写要封锁的IP地址"));
    }

    // 正则检查，检查传入的IP地址是否符合ipv4规范
    let re = Regex::new(r"^(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)$").unwrap();

    let mut blocked_ip_vec: Vec<String> = Vec::new();

    for i in write_ip {
        if let Some(ip_addr) = i.ipv4 {
            blocked_ip_vec.push(ip_addr.clone());
        }
    }

    // 再检查一下，看看是不是传的都是空的数据
    if blocked_ip_vec.is_empty() {
        return Ok(HttpResponse::Ok().json("传入的IP数量为零,请填写要封锁的IP地址"));
    }

    // 现在的封锁IP数据列表
    let ip_stored: Vec<BlockedIp> = sqlx::query_as("SELECT id, ipv4 FROM blocked_ip")
        .fetch_all(db_pool)
        .await?;
    let mut ip: Vec<String> = Vec::new();
    for i in ip_stored {
        ip.push(i.ipv4);
    }

    // 如果不匹配就返回错误
    for i in blocked_ip_vec {
        if !re.is_match(i.as_str()) {
            return Err(MyError::InvalidInput(
                format!("输入的{}地址并不是符合ipv4规范的IP地址", i).to_string(),
            ));
        }
        // 检查一下这个IP地址是否已经存在
        if ip.contains(&i) {
            return Err(MyError::DBError(
                format!("{}已经存在于数据库中了", i).to_string(),
            ));
        }

        // 将IP地址存入到数据库中
        let _ = sqlx::query("INSERT INTO blocked_ip (ipv4) VALUES ($1)")
            .bind(i)
            .execute(db_pool)
            .await?;
    }

    kill_and_restart(state.clone()).await?;

    Ok(HttpResponse::Ok().json("ipv4成批地添加成功, 幸福的时代即将到来".to_string()))
}
