use serde::{Deserialize,Serialize};

#[derive(Default,Deserialize)]
pub(crate) struct ListQuery {
    current: Option<String>,
    #[serde(rename="pageSize")]
    page_size: Option<String>,
    status: Option<String>,
    name: Option<String>,
}

pub(crate) struct PageRequest {
    pub offset: i64,
    pub limit: i64,
    pub status: Option<i64>,
    pub name: Option<String>,
    pub name_pattern: Option<String>,
}

#[derive(Serialize)]
pub(crate) struct Page<T> {
    pub total: usize,
    pub data: Vec<T>,
    pub code: u8,
}

impl ListQuery {
    pub fn parse(self) -> Result<PageRequest,&'static str> {
        let current = self.current.map(|value|value.parse::<u64>()).transpose().map_err(|_|"invalid_current")?.unwrap_or(1);
        let limit = self.page_size.map(|value|value.parse::<u64>()).transpose().map_err(|_|"invalid_page_size")?.unwrap_or(100);
        if current == 0 { return Err("invalid_current"); }
        if !(1..=100).contains(&limit) { return Err("invalid_page_size"); }
        let offset = (current-1).checked_mul(limit).and_then(|offset|i64::try_from(offset).ok()).ok_or("invalid_current")?;
        let status = self.status.map(|value|value.parse::<i64>()).transpose().map_err(|_|"invalid_status")?;
        if status.is_some_and(|status| !matches!(status,0|1)) { return Err("invalid_status"); }
        if self.name.as_ref().is_some_and(|name|name.len()>256) { return Err("invalid_name_filter"); }
        let name = self.name.filter(|name|!name.is_empty());
        let name_pattern = name.as_ref().map(|name|format!("%{}%",name.replace('\\',"\\\\").replace('%',"\\%").replace('_',"\\_")));
        Ok(PageRequest { offset,limit:limit as i64,status,name,name_pattern })
    }
}
