pub mod files {
    use serde::de::DeserializeOwned;
    use std::fs::File;
    use std::io;
    use std::io::{BufReader, prelude::*};
    use std::path::Path;
    use toml;

    pub fn check_if_file_exists(file_path: &String) -> bool {
        /*
         *  Check if file exists in given directory.
         *
         *  Arguments:
         *      file_path: This should be a full path to the file.
         *
         *  Returns:
         *      Return true if exists, false otherwise.
         */
        Path::new(file_path).exists()
    }

    pub fn read_to_bytes(file_path: &Path) -> Vec<u8> {
        /*
         *  Read the entire file to the vector of bytes.
         *
         *  Arguments:
         *      file_path: This should be preprocessed file path.
         *
         *  Returns:
         *      Returns the vector of bytes, if it failed, it returns the empty vector.
         */
        let file_read_result = File::open(file_path);
        let file_buffered_content = match file_read_result {
            Ok(file) => file,
            Err(_) => return Vec::new(),
        };
        let mut buf_reader = BufReader::new(file_buffered_content);
        let mut content: Vec<u8> = Vec::new();
        let content_buf_sz = buf_reader.read_to_end(&mut content);
        match content_buf_sz {
            Ok(n) => return if n > 0 { content } else { Vec::new() },
            Err(_) => return Vec::new(),
        };
    }

    pub fn read_to_str(file_path: &Path) -> Result<String, io::Error> {
        /*
         *  Read the entire file to the string.
         *
         *  Arguments:
         *      file_path: This should be preprocessed file path.
         *
         *  Returns:
         *      Returns the result if suceeds with a String, otherwise error.
         */
        let file_state = File::open(file_path);
        let file_content = match file_state {
            Ok(file) => file,
            Err(error) => return Err(error),
        };
        let mut buf = BufReader::new(file_content);
        let mut contents = String::new();
        let content = buf.read_to_string(&mut contents);
        match content {
            Ok(_) => return Ok(contents),
            Err(error) => return Err(error),
        };
    }

    pub fn read_toml<T: DeserializeOwned>(file_path: &Path) -> Result<T, io::Error> {
        /*
         *  Read TOML file to the String.
         *
         *  Arguments:
         *      file_path: Preprocessed file path.
         *
         *  Returns:
         *      Returns TOML if succeeds otherwise Error.
         */
        let data = read_to_str(&file_path).unwrap();
        let toml = toml::from_str(&data).unwrap();
        return Ok(toml);
    }
}

pub mod buffers {
    use std::{cmp, error::Error};
    use tokio::net::TcpStream;
    pub mod constants {
        /* Ascii decimals */
        pub const NEWLINE: u8 = 10;
        pub const CR: u8 = 13;
        pub const SPACE: u8 = 32;
        /* Content-Length: */
        pub const CONTENT_LENGTH_FIELD: &[u8] = &[
            67, 111, 110, 116, 101, 110, 116, 45, 76, 101, 110, 103, 116, 104, 58, 32,
        ];
        /* Get */
        pub const GET_REQUEST: &[u8] = &[71, 69, 84];
        /* Post */
        pub const POST_REQUEST: &[u8] = &[80, 79, 83, 84];
        /* site_not_found.html */
        pub const SITE_NOT_FOUND: &[u8] = &[
            115, 105, 116, 101, 95, 110, 111, 116, 95, 102, 111, 117, 110, 100, 46, 104, 116, 109,
            108,
        ];
    }

    pub fn read_tcpstream(stream: &TcpStream) -> Result<Vec<u8>, Box<dyn Error>> {
        /*
         *  Read TCPStream to the Vector buffer. The maximum allowed size
         *  is 8192 bytes.
         *
         *  Arguments:
         *      stream: Stream that will be read into the vector buffer.
         *
         *  Returns:
         *      Returns either the vector or an error if failed.
         */

        let mut buffered: Vec<u8> = Vec::with_capacity(8192);
        match stream.try_read_buf(&mut buffered) {
            Ok(sz) => {
                println!("[INFO] Read {sz} bytes");
                return Ok(buffered);
            }
            Err(e) => {
                return Err(e.into());
            }
        }
    }

    pub fn extract_number(buffer: &[u8]) -> i64 {
        /*
         *  Extract the number in the buffer. It tries to extract until it
         *  hits the \r\n sequence. (EOL)
         *
         *  Arguments:
         *      buffer: The buffer, which must be preprocessed.
         *
         *  Returns:
         *      Returns either the number or 0 if failed.
         */

        // TODO: Should do better handling the verification.
        /* Initialize flags to handle sequences */
        let mut cr_flag: bool = false;
        let mut nl_flag: bool = false;
        let mut num_bytes: Vec<i64> = Vec::new();

        for el in buffer {
            cr_flag = if *el == constants::CR { true } else { false };
            nl_flag = if *el == constants::NEWLINE {
                true
            } else {
                false
            };

            /* If true, then we are EOL, so there is no more numbers */
            if cr_flag || nl_flag {
                break;
            }
            /* If the previous conditions is false, we add numbers to the buf */
            let num_in_byte: i64 = (*el).into();
            num_bytes.push(num_in_byte);
        }

        let mut magnitude: i64 = 1;
        return num_bytes
            .iter()
            .rev()
            .map(|&x: &i64| {
                let res: i64 = (x - 48) * magnitude;
                magnitude *= 10;
                return res;
            })
            .sum();
    }
    pub fn find_in_buffer(buffer: &Vec<u8>, pattern: &[u8]) -> usize {
        /*
         *  Find index in a buffer with given pattern. It might be (is) used
         *  for preprocessing. It is based on Rabin-Karp algorithm.
         *
         *  Arguments:
         *      buffer: The buffer that you want to search.
         *      pattern: The pattern that needs to be found.
         *
         *  Returns:
         *      Returns the index on which the pattern starts. If the pattern
         *      doesn't exist, maximum number is returned.
         */

        /* Declare helper variables */
        let pattern_sz: usize = pattern.len();
        let buffer_sz: usize = buffer.len();
        let prime: i64 = 31;
        let large_prime: i64 = 1_000_000_009;

        /* Must explicitly cast */
        let capacity: i64 = cmp::max(pattern_sz as i64, buffer_sz as i64);

        /* Ignore the value initialization up to capcity + 1, the one that
         * matters is the first entry which must be 1, from the first index up to
         * the end, it will be calculated and overriden.
         */
        let mut powers: Vec<i64> = Vec::from_iter(1..capacity + 1);
        for idx in 1..capacity as usize {
            powers[idx] = (powers[idx - 1] * prime) % large_prime;
        }

        /* Again ignore the initialization - it is the same as the vector powers,
         * what matters is only first entry which must be initialized to the 0
         * */
        let mut buffer_hash_prefixes: Vec<i64> = Vec::from_iter(0..(buffer_sz + 1) as i64);
        for idx in 0..buffer_sz {
            buffer_hash_prefixes[idx + 1] =
                (buffer_hash_prefixes[idx] + buffer[idx] as i64 * powers[idx]) % large_prime;
        }

        let mut hashed_pattern: i64 = 0;
        for idx in 0..pattern_sz {
            hashed_pattern = hashed_pattern + (pattern[idx] as i64 * powers[idx]) % large_prime;
        }

        for idx in 0..(buffer_sz - pattern_sz + 1) {
            let current_hash: i64 = (buffer_hash_prefixes[idx + pattern_sz] + large_prime
                - buffer_hash_prefixes[idx])
                % large_prime;
            if current_hash == hashed_pattern * powers[idx] % large_prime {
                return idx;
            }
        }
        /* If the above for loops fails, return this. */
        return usize::MAX;
    }
}

mod tests {
    use super::buffers::{constants::CONTENT_LENGTH_FIELD, find_in_buffer};

    #[test]
    fn find_in_buffer_test() {
        /* Test 1 */
        let vec_content_buf: Vec<u8> = Vec::from(CONTENT_LENGTH_FIELD);
        let content_pos = find_in_buffer(&vec_content_buf, CONTENT_LENGTH_FIELD);
        assert_eq!(content_pos, 0);

        /* Test 2 */
        const POST_REQUEST: &[u8] = b"POST /api/data HTTP/1.1\r\n\
            Host: example.com\r\n\
            Content-Type: application/json\r\n\
            Content-Length: 27\r\n\
            \r\n\
            {\"key\":\"value\",\"number\":42}";

        let vec_post_buf: Vec<u8> = Vec::from(POST_REQUEST);
        let post_pos = find_in_buffer(&vec_post_buf, CONTENT_LENGTH_FIELD);
        assert_eq!(post_pos, 76);
    }
}
