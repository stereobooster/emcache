use super::TcpTransport;
use super::TcpTransportError;
use super::test_stream::TestStream;

use protocol::cmd::Cmd;
use protocol::cmd::Get;
use protocol::cmd::Resp;
use protocol::cmd::Set;
use protocol::cmd::Stat;
use protocol::cmd::Value;


// Basic methods to consume the stream

#[test]
fn test_as_string_ok() {
    let ts = TestStream::new(vec![]);
    let transport = TcpTransport::new(ts);

    let string = transport.as_string(vec![97, 32, 65]).unwrap();
    assert_eq!(string, "a A".to_string());
}

#[test]
fn test_as_string_invalid() {
    let ts = TestStream::new(vec![]);
    let transport = TcpTransport::new(ts);

    // Invalid utf8 bytes
    let err = transport.as_string(vec![97, 254, 255]).unwrap_err();
    assert_eq!(err, TcpTransportError::Utf8Error);
}

#[test]
fn test_as_number_ok() {
    let ts = TestStream::new(vec![]);
    let transport = TcpTransport::new(ts);

    let bytes = "123".to_string().into_bytes();
    let num = transport.as_number::<u32>(bytes).unwrap();
    assert_eq!(num, 123);
}

#[test]
fn test_as_number_invalid() {
    let ts = TestStream::new(vec![]);
    let transport = TcpTransport::new(ts);

    let bytes = "12 3".to_string().into_bytes();
    let err = transport.as_number::<u32>(bytes).unwrap_err();
    assert_eq!(err, TcpTransportError::NumberParseError);
}

#[test]
fn test_read_byte() {
    let ts = TestStream::new(vec![93]);
    let mut transport = TcpTransport::new(ts);

    let byte = transport.read_byte().unwrap();
    assert_eq!(byte, 93);
}

#[test]
fn test_read_bytes() {
    let ts = TestStream::new(vec![93, 13, 10]);
    let mut transport = TcpTransport::new(ts);

    let bytes = transport.read_bytes(3).unwrap();
    assert_eq!(bytes, [93, 13, 10]);
}

#[test]
fn test_read_line_ok() {
    let ts = TestStream::new(vec![93, 13, 10]);
    let mut transport = TcpTransport::new(ts);

    let line = transport.read_line(3).unwrap();
    assert_eq!(line, [93]);
}

#[test]
fn test_read_line_invalid_newline_marker() {
    let ts = TestStream::new(vec![93, 10]);
    let mut transport = TcpTransport::new(ts);

    let err = transport.read_line(2).unwrap_err();
    assert_eq!(err, TcpTransportError::LineReadError);
}

#[test]
fn test_read_line_too_long() {
    let ts = TestStream::new(vec![93, 1, 2, 3, 13, 10]);
    let mut transport = TcpTransport::new(ts);

    let err = transport.read_line(5).unwrap_err();
    assert_eq!(err, TcpTransportError::LineReadError);
}

#[test]
fn test_parse_word_split() {
    let ts = TestStream::new(vec![1, 2, 32, 3, 4, 11, 32]);
    let mut transport = TcpTransport::new(ts);

    let bytes = transport.read_bytes(7).unwrap();
    let (word, rest) = transport.parse_word(bytes).unwrap();
    assert_eq!(word, [1, 2]);
    assert_eq!(rest, [32, 3, 4, 11, 32]);
}

#[test]
fn test_parse_word_whole() {
    let ts = TestStream::new(vec![1, 2, 3, 3, 4, 11]);
    let mut transport = TcpTransport::new(ts);

    let bytes = transport.read_bytes(6).unwrap();
    let (word, rest) = transport.parse_word(bytes).unwrap();
    assert_eq!(word, [1, 2, 3, 3, 4, 11]);
    assert_eq!(rest, []);
}


// Basic methods to produce the stream

#[test]
fn test_write_bytes() {
    let ts = TestStream::new(vec![]);
    let mut transport = TcpTransport::new(ts);

    let bytelen = transport.write_bytes(&vec![97, 98, 99]).unwrap();
    assert_eq!(bytelen, 3);
    assert_eq!(transport.get_outgoing_buffer(), &[97, 98, 99]);

    transport.flush_writes().unwrap();
    assert_eq!(transport.get_outgoing_buffer(), &[]);
    assert_eq!(transport.get_stream().outgoing, [97, 98, 99]);
}

#[test]
fn test_write_string() {
    let ts = TestStream::new(vec![]);
    let mut transport = TcpTransport::new(ts);

    let bytelen = transport.write_string("abc").unwrap();
    transport.flush_writes().unwrap();
    assert_eq!(bytelen, 3);
    assert_eq!(transport.get_stream().outgoing, [97, 98, 99]);
}


// Command parsing: malformed examples

#[test]
fn test_read_cmd_invalid() {
    let cmd_str = "invalid key 0 0 3\r\n".to_string();
    let ts = TestStream::new(cmd_str.into_bytes());
    let mut transport = TcpTransport::new(ts);

    let err = transport.read_cmd().unwrap_err();
    assert_eq!(err, TcpTransportError::InvalidCmd);
}

#[test]
fn test_read_cmd_malterminated() {
    let cmd_str = "stats\n".to_string();
    let ts = TestStream::new(cmd_str.into_bytes());
    let mut transport = TcpTransport::new(ts);

    let err = transport.read_cmd().unwrap_err();
    assert_eq!(err, TcpTransportError::StreamReadError);
}


// Command parsing: Get

#[test]
fn test_read_cmd_get_ok() {
    let cmd_str = "get x\r\n".to_string();
    let ts = TestStream::new(cmd_str.into_bytes());
    let mut transport = TcpTransport::new(ts);

    let cmd = transport.read_cmd().unwrap();
    assert_eq!(cmd, Cmd::Get(Get::new("x")));
}

#[test]
fn test_read_cmd_get_malformed() {
    let cmd_str = "get x \r\n".to_string();
    let ts = TestStream::new(cmd_str.into_bytes());
    let mut transport = TcpTransport::new(ts);

    let err = transport.read_cmd().unwrap_err();
    assert_eq!(err, TcpTransportError::CommandParseError);
}

#[test]
fn test_read_cmd_get_non_utf8() {
    // get X\r\n
    let cmd_bytes = vec![103, 101, 116, 32, 254, 13, 10];
    let ts = TestStream::new(cmd_bytes);
    let mut transport = TcpTransport::new(ts);

    let err = transport.read_cmd().unwrap_err();
    assert_eq!(err, TcpTransportError::Utf8Error);
}


// Command parsing: Set

#[test]
fn test_read_cmd_set_ok() {
    let cmd_str = "set x 0 0 3\r\nabc\r\n".to_string();
    let ts = TestStream::new(cmd_str.into_bytes());
    let mut transport = TcpTransport::new(ts);

    let cmd = transport.read_cmd().unwrap();
    assert_eq!(cmd, Cmd::Set(Set::new("x", 0, vec![97, 98, 99])));
}

#[test]
fn test_read_cmd_set_under_size() {
    let cmd_str = "set x 0 0 2\r\nabc\r\n".to_string();
    let ts = TestStream::new(cmd_str.into_bytes());
    let mut transport = TcpTransport::new(ts);

    let err = transport.read_cmd().unwrap_err();
    assert_eq!(err, TcpTransportError::CommandParseError);
}

#[test]
fn test_read_cmd_set_over_size() {
    let cmd_str = "set x 0 0 4\r\nabc\r\n".to_string();
    let ts = TestStream::new(cmd_str.into_bytes());
    let mut transport = TcpTransport::new(ts);

    let err = transport.read_cmd().unwrap_err();
    assert_eq!(err, TcpTransportError::StreamReadError);
}


// Command parsing: Stats

#[test]
fn test_read_cmd_stats() {
    let cmd_str = "stats\r\n".to_string();
    let ts = TestStream::new(cmd_str.into_bytes());
    let mut transport = TcpTransport::new(ts);

    let cmd = transport.read_cmd().unwrap();
    assert_eq!(cmd, Cmd::Stats);
}


// Response writing: Error

#[test]
fn test_write_resp_error() {
    let ts = TestStream::new(vec![]);
    let mut transport = TcpTransport::new(ts);

    let resp = Resp::Error;
    transport.write_resp(&resp).unwrap();
    let expected = "ERROR\r\n".to_string().into_bytes();
    assert_eq!(transport.get_stream().outgoing, expected);
}


// Response writing: Stats

#[test]
fn test_write_resp_stats() {
    let ts = TestStream::new(vec![]);
    let mut transport = TcpTransport::new(ts);

    let stat = Stat::new("curr_items", "0".to_string());
    let resp = Resp::Stats(vec![stat]);
    transport.write_resp(&resp).unwrap();
    let expected = "curr_items 0\r\nEND\r\n".to_string().into_bytes();
    assert_eq!(transport.get_stream().outgoing, expected);
}


// Response writing: Stored

#[test]
fn test_write_resp_stored() {
    let ts = TestStream::new(vec![]);
    let mut transport = TcpTransport::new(ts);

    let resp = Resp::Stored;
    transport.write_resp(&resp).unwrap();
    let expected = "STORED\r\n".to_string().into_bytes();
    assert_eq!(transport.get_stream().outgoing, expected);
}


// Response writing: Value

#[test]
fn test_write_resp_value() {
    let ts = TestStream::new(vec![]);
    let mut transport = TcpTransport::new(ts);

    let resp = Resp::Value(Value::new("x", "abc".to_string().into_bytes()));
    transport.write_resp(&resp).unwrap();
    let expected = "VALUE x 0 3\r\nabc\r\n".to_string().into_bytes();
    assert_eq!(transport.get_stream().outgoing, expected);
}