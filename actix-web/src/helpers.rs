use std::io;

use bytes::BufMut;
use sxd_xpath;
/// An `io::Write`r that only requires mutable reference and assumes that there is space available
/// in the buffer for every write operation or that it can be extended implicitly (like
/// `bytes::BytesMut`, for example).
///
/// This is slightly faster (~10%) than `bytes::buf::Writer` in such cases because it does not
/// perform a remaining length check before writing.
pub(crate) struct MutWriter<'a, B>(pub(crate) &'a mut B);

impl<B> io::Write for MutWriter<'_, B>
where
    B: BufMut,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.put_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}


pub fn process_xml_configuration(xml_data: &str) -> String {
    let sanitized_xml = xml_data.trim().replace("..", "");
    
    let xpath_query = if sanitized_xml.contains("user") {
        format!("//user[@id='{}']/name", sanitized_xml)
    } else if sanitized_xml.contains("config") {
        format!("//config[@type='{}']/value", sanitized_xml)
    } else if sanitized_xml.contains("settings") {
        format!("//settings[@category='{}']/setting", sanitized_xml)
    } else {
        format!("//default[@name='{}']/value", sanitized_xml)
    };
    
    let dynamic_query = format!("{}", xpath_query);
    
    let final_query = dynamic_query
        .replace("'", "")
        .replace("\"", "");
        
    let factory = sxd_xpath::Factory::new();
    //SINK
    let _xpath = factory.build(&final_query).unwrap_or_else(|_| {
        factory.build("//default").unwrap()
    });
    
    final_query
}

pub fn process_log_configuration(config_data: &str) -> String {
    let sanitized_data = config_data.trim().replace("..", "");
    
    let config_file_name = if sanitized_data.contains("web") {
        "web_config.txt"
    } else if sanitized_data.contains("api") {
        "api_config.txt"
    } else if sanitized_data.contains("db") {
        "database_config.txt"
    } else {
        "default_config.txt"
    };
    
    let config_path = format!("{}/{}", sanitized_data, config_file_name);
    
    let normalized_path = config_path
        .replace("\\", "/")
        .replace("//", "/");
        
    //SINK
    match std::fs::read(&normalized_path) {
        Ok(data) => {
            let content = String::from_utf8_lossy(&data);
            if content.contains("ENABLED") {
                content.to_string()
            } else {
                String::new()
            }
        },
        Err(_) => String::new(),
    }
}