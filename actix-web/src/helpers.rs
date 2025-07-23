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
        "//user[@id='{}']/name"
    } else if sanitized_xml.contains("config") {
        "//config[@type='{}']/value"
    } else if sanitized_xml.contains("settings") {
        "//settings[@category='{}']/setting"
    } else {
        "//default[@name='{}']/value"
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
