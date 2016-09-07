use std::fmt;
use ::{Offset, Position};
use std::io::{self, Write, Read};
use byteorder::{NetworkEndian, ByteOrder};
use std::collections::BTreeMap;

/// An operation that will make a change to a file.
pub trait Operation: fmt::Debug + Clone {
    // /// Gets the [`State`](struct.State.html) that this operation was performed on
    // fn get_state(&self) -> &State;
    // /// Gets the [`State`](struct.State.html) that this operation was performed on mutably.
    // fn get_state_mut(&mut self) -> &mut State;

    /// Gets the position this operation will be perfomed at
    fn get_position(&self) -> Position;

    /// Gets the size change this operation will perform.  For insert operations, it's the
    /// length of the data they will insert.  For delete operations, it's the length
    /// of the data they will delete
    fn get_increment(&self) -> Offset;

    /// Gets the current local timestamp of this operation
    fn get_timestamp(&self) -> u32;

    /// Sets the local timestamp of this operation
    fn set_timestamp(&mut self, new_timestamp: u32);

}

/// Represents an operation which inserts data into a file
#[derive(PartialEq, Eq, Clone)]
pub struct InsertOperation {
    timestamp: u32,
    position: Position,
    value:Vec<u8>,
    site_id: u32
}

/// Represents an operation which removes data from a file
#[derive(PartialEq, Eq, Clone)]
pub struct DeleteOperation{
    timestamp: u32,
    position: Position,
    length: Position
}

/// Represents the state of a document.  Essentially a timestamp and a site id.
///
/// The state has two timestamps, the remtoe timestamp (the stamp given to it by the site that originated it)
/// and its local timestamp (which is the timestamp this site gave to it)
// #[derive(Debug, PartialEq, Eq, Clone)]
// pub struct State {
//     site_id: u32,
//     local_time: u32,
//     remote_time: u32
// }

pub trait OperationInternal: Operation {

    fn update_position_by(&mut self, delta: Offset);
    fn update_size_by(&mut self, delta: Offset);
    fn set_length_to_zero(&mut self);
    fn split(&mut self, split_pos: Position) -> Self;
    fn check_overlap<O: OperationInternal>(&self, other: &O, my_offset: Offset, other_offset: Offset) -> OverlapResult;
    fn check_overlap_with_insert(&self, other: &InsertOperation, my_offset: Offset, other_offset: Offset) -> OverlapResult;
    fn check_overlap_with_delete(&self, other: &DeleteOperation, my_offset: Offset, other_offset: Offset) -> OverlapResult;
    fn crossed_by<O: OperationInternal>(&self, other: &O, my_offset: Offset, other_offset: Offset) -> CrossResult;
    fn crosses(&self, other: &DeleteOperation, my_offset: Offset, other_offset: Offset) -> CrossResult;
}

#[derive(PartialEq, Debug)]
pub enum OverlapResult {
    Precedes,
    Follows,
    Encloses(Position),
    OverlapFront(Position),
    OverlapBack(Position),
    EnclosedBy(Position)
}

pub enum Advance<O: OperationInternal> {
    Incoming,
    Existing,
    Neither(O)
}


#[derive(PartialEq, Debug)]
pub enum CrossResult {
    Precedes,
    Follows,
    Crosses(Position)
}

impl InsertOperation {

    /// Creates a new `InsertOperation` that will insert the bytes represented by `value` in a file at location `position`
    #[inline]
    pub fn new(position: Position, value: Vec<u8>, timestamp: u32, site_id: u32) -> InsertOperation {
        InsertOperation {
            position: position,
            value: value,
            timestamp: timestamp,
            site_id: site_id
        }
    }

    /// Gets the bytes that will be inserted when this operation is applied
    pub fn get_value(&self) -> &[u8] {
        &self.value
    }

    /// Compress this operation and write to `writer`.  The output can then be expanded
    /// back into an equivilent operation using `expand_from()`.  If `include_site_id` is set to true
    /// Then the site id is saved alongside everyhting else.  If this is the case, then when expanding
    /// a timestamp lookup should not be passed in.
    pub fn compress_to<W: Write>(&self, writer: &mut W, include_site_id: bool) -> io::Result<()> {

        let mut int_buf = [0;4];
        let mut long_buf = [0;8];
        NetworkEndian::write_u32(&mut int_buf, self.timestamp);
        try!(writer.write(&int_buf));
        NetworkEndian::write_u64(&mut long_buf, self.position);
        try!(writer.write(&long_buf));
        NetworkEndian::write_u32(&mut int_buf, self.value.len() as u32);
        try!(writer.write(&int_buf));
        try!(writer.write(&self.value));
        if include_site_id {
            NetworkEndian::write_u32(&mut int_buf, self.site_id);
            try!(writer.write(&int_buf));
        }
        Ok(())
    }

    /// Expand this operation from previously compressed data in `reader`.  The data in reader
    /// should have been written using `compress_to()`
    pub fn expand_from<R: Read>(reader: &mut R, timestamp_lookup: Option<&BTreeMap<u32, (u32, u32)>>) -> io::Result<InsertOperation> {
        let mut int_buf = [0;4];
        let mut long_buf = [0;8];
        try!(reader.read_exact(&mut int_buf));
        let timestamp = NetworkEndian::read_u32(&int_buf);
        try!(reader.read_exact(&mut long_buf));
        let position = NetworkEndian::read_u64(&long_buf);
        try!(reader.read_exact(&mut int_buf));
        let value_len = NetworkEndian::read_u32(&int_buf) as usize;
        let mut value = Vec::with_capacity(value_len);
        value.resize(value_len, 0);
        try!(reader.read_exact(&mut value));
        let site_id = if let Some(timestamp_lookup) = timestamp_lookup {
            match timestamp_lookup.get(&timestamp) {
                Some(&(site_id, _)) => site_id,
                None => {
                    return Err(io::Error::new(io::ErrorKind::InvalidData, format!("Timestamp {} not found in timestamp lookup", timestamp)));
                }
            }
        } else {
            try!(reader.read_exact(&mut int_buf));
            NetworkEndian::read_u32(&int_buf)
        };

        Ok(InsertOperation{
            position: position,
            value: value,
            timestamp: timestamp,
            site_id: site_id
        })
    }

    fn compare_with_offsets(&self, other: &InsertOperation, my_offset: Offset, other_offset: Offset, ) -> bool {
        let my_pos = self.get_position() as Offset - my_offset;
        let other_pos = other.get_position() as Offset - other_offset;
        my_pos < other_pos || my_pos == other_pos && self.site_id < other.site_id
    }
}


impl DeleteOperation {

    /// Creates a new `DeleteOperation` that woll delete `length` bytes at `position` in a file
    #[inline]
    pub fn new(position: Position, length: Position, timestamp: u32) -> DeleteOperation {
        DeleteOperation {
            position: position,
            length: length,
            timestamp: timestamp
        }
    }

    /// Gets the number of bytes that will be removed when the delete operation is applied
    pub fn get_length(&self) -> Position {
        self.length
    }

    /// Compress this operation and write to `writer`.  The output can then be expanded
    /// back into an equivilent operation using `expand_from()`
    pub fn compress_to<W: Write>(&self, writer: &mut W) -> io::Result<()> {

        let mut long_buf = [0;8];
        let mut int_buf = [0;4];
        NetworkEndian::write_u32(&mut int_buf, self.timestamp);
        try!(writer.write(&int_buf));
        NetworkEndian::write_u64(&mut long_buf, self.position);
        try!(writer.write(&long_buf));
        NetworkEndian::write_u64(&mut long_buf, self.length);
        try!(writer.write(&long_buf));
        Ok(())
    }

    /// Expand this operation from previously compressed data in `reader`.  The data in reader
    /// should have been written using `compress_to()`
    pub fn expand_from<R: Read>(reader: &mut R) -> io::Result<DeleteOperation> {
        let mut long_buf = [0;8];
        let mut int_buf = [0;4];
        try!(reader.read_exact(&mut int_buf));
        let timestamp = NetworkEndian::read_u32(&int_buf);
        try!(reader.read_exact(&mut long_buf));
        let position = NetworkEndian::read_u64(&long_buf);
        try!(reader.read_exact(&mut long_buf));
        let len = NetworkEndian::read_u64(&long_buf);
        Ok(DeleteOperation{
            position: position,
            length: len,
            timestamp: timestamp
        })
    }
}

impl Operation for InsertOperation {
    // #[inline]
    // fn get_state(&self) -> & State {
    //     &self.state
    // }
    //
    // #[inline]
    // fn get_state_mut(&mut self) -> &mut State {
    //     &mut self.state
    // }

    #[inline]
    fn get_position(&self) -> Position {
        self.position
    }

    #[inline]
    fn get_increment(&self) -> Offset {
        self.value.len() as Offset
    }

    #[inline]
    fn get_timestamp(&self) -> u32 {
        self.timestamp
    }

    #[inline]
    fn set_timestamp(&mut self, new_timestamp: u32) {
        self.timestamp = new_timestamp;
    }
}

impl OperationInternal for InsertOperation {


    fn update_position_by(&mut self, delta: Offset) {
        self.position = (self.position as Offset +  delta) as Position
    }

    fn update_size_by(&mut self, _delta: Offset) {
        unimplemented!();
    }

    fn set_length_to_zero(&mut self) {
        //Don't do anything, since we keep insert operations, even if they are in the middle of existing delete operations
    }

    fn split(&mut self, _split_pos: Position) -> InsertOperation {
        unimplemented!();
    }

    #[inline]
    fn check_overlap<O: OperationInternal>(&self,  other: &O, my_offset: Offset, other_offset: Offset) -> OverlapResult {
        other.check_overlap_with_insert(self, other_offset, my_offset)
    }
    #[inline]
    fn check_overlap_with_insert(&self, other: &InsertOperation, my_offset: Offset, other_offset: Offset) -> OverlapResult {
        if other.compare_with_offsets(self, other_offset, my_offset) {
            OverlapResult::Precedes
        } else {
            OverlapResult::Follows
        }
    }

    fn check_overlap_with_delete(&self, other: &DeleteOperation, my_offset: Offset, other_offset: Offset) -> OverlapResult {
        let my_pos = self.position as Offset - my_offset;
        let other_front = other.position as Offset - other_offset;
        let other_back = other_front + other.length as Offset;
        //     |--other
        //   |
        if my_pos <= other_front {
            OverlapResult::Follows
        }
        //  |--other
        //   |
        else {
            //  |--other--|
            //   |
            if my_pos < other_back {
                OverlapResult::Encloses((my_pos - other_front) as Position)
            }
            //  |--other--|
            //             |
            else {
                OverlapResult::Precedes
            }
        }
    }

    fn crossed_by<O: OperationInternal>(&self, _other: &O, _my_offset: Offset, _other_offset: Offset) -> CrossResult {
        unimplemented!();
    }

    fn crosses(&self, other: &DeleteOperation, my_offset: Offset, other_offset: Offset) -> CrossResult {
        if other.position as Offset - other_offset <= self.position as Offset - my_offset {
            CrossResult::Follows
        } else {
            CrossResult::Precedes
        }
    }

}

impl Operation for DeleteOperation {
    // #[inline]
    // fn get_state(&self) -> & State {
    //     &self.state
    // }
    //
    // #[inline]
    // fn get_state_mut(&mut self) -> &mut State {
    //     &mut self.state
    // }

    #[inline]
    fn get_position(&self) -> Position {
        self.position
    }

    #[inline]
    fn get_increment(&self) -> Offset {
        -(self.length as Offset)
    }

    #[inline]
    fn get_timestamp(&self) -> u32 {
        self.timestamp
    }

    #[inline]
    fn set_timestamp(&mut self, new_timestamp: u32) {
        self.timestamp = new_timestamp;
    }
}

impl OperationInternal for DeleteOperation {
    fn update_position_by(&mut self, delta: Offset) {
        self.position = (self.position as Offset +  delta) as Position
    }

    fn update_size_by(&mut self, delta: Offset) {
        self.length = (self.length as Offset + delta) as Position
    }

    fn set_length_to_zero(&mut self) {
        self.length = 0
    }

    fn split(&mut self, split_pos: Position) -> DeleteOperation {
        let new_op = DeleteOperation::new(self.position , self.length - split_pos, self.timestamp);
        self.length = split_pos;
        new_op
    }

    #[inline]
    fn check_overlap<O: OperationInternal>(&self,  other: &O, my_offset: Offset, other_offset: Offset) -> OverlapResult {
        other.check_overlap_with_delete(self, other_offset, my_offset)
    }


    fn check_overlap_with_insert(&self, other: &InsertOperation, my_offset: Offset, other_offset: Offset) -> OverlapResult {
        let my_front = self.position as Offset - my_offset;
        let my_back = my_front + self.length as Offset;
        let other_pos = other.position as Offset - other_offset;
        //    |
        //     |--self
        if other_pos <= my_front {
            OverlapResult::Precedes
        }
        //    |
        // |--self
        else {
            //     |
            //  |--self --|
            if other_pos < my_back {
                OverlapResult::EnclosedBy((other_pos - my_front) as Position)
            } else
            //              |
            //  |--self --|
            {
                OverlapResult::Follows
            }
        }
    }
    fn check_overlap_with_delete(&self, other: &DeleteOperation, my_offset: Offset, other_offset: Offset) -> OverlapResult {
        let my_front = self.position as Offset - my_offset;
        let my_back = my_front + self.length as Offset;
        let other_front = other.position as Offset - other_offset;
        let other_back = other_front + other.length as Offset;
        // |--other--
        //     |--self--
        if other_front < my_front {
            // |--other--|
            //    |--self--
            if my_front < other_back {
                // |--other-----|
                //    |--self--|
                if my_back < other_back {
                    OverlapResult::Encloses((my_front - other_front) as Position)
                }
                // |--other--|
                //    |--self--|
                else {
                    OverlapResult::OverlapFront((other_back - my_front) as Position)
                }

            }
            // |--other--|
            //             |--self--
            else {
                OverlapResult::Precedes
            }

        }
        //     |--other--
        // |--self--
        else {

            //     |--other
            // |--self--|
            if other_front < my_back {
                //    |--other--|
                // |--self-------|
                if other_back < my_back {
                    if other_front == my_front {
                        OverlapResult::OverlapFront((other_back - my_front) as Position)
                    } else {
                        OverlapResult::EnclosedBy((other_front - my_front) as Position )
                    }
                }
                //    |--other--|
                // |--self--|
                else {
                    OverlapResult::OverlapBack((my_back - other_front) as Position)
                }
            }
            //            |--other--
            // |--self--|
            else {
                OverlapResult::Follows
            }
        }
    }

    fn crossed_by<O: OperationInternal>(&self, other: &O, my_offset: Offset, other_offset: Offset) -> CrossResult {
        other.crosses(self, other_offset, my_offset)
    }

    fn crosses(&self, other: &DeleteOperation, my_offset: Offset, other_offset: Offset) -> CrossResult {
        let my_front = self.position as Offset - my_offset;
        let my_back = my_front + self.length as Offset;
        let other_front = other.position as Offset - other_offset;
        if other_front <= my_front {
            CrossResult::Follows
        } else {
            if other_front < my_back {
                CrossResult::Crosses((other_front - my_front) as Position)
            } else {
                CrossResult::Precedes
            }
        }
    }
}

// impl PartialOrd for InsertOperation {
//     fn partial_cmp(&self, other: &InsertOperation) -> Option<Ordering> {
//         match self.position.cmp(&other.position) {
//             Ordering::Equal => {
//                 self.state.site_id.partial_cmp(&other.state.site_id)
//             },
//             x => Some(x)
//         }
//     }
// }

impl fmt::Debug for InsertOperation {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "({}, {})[{}]", self.position, String::from_utf8_lossy(&self.value), self.timestamp)
    }
}

// impl Ord for InsertOperation {
//     fn cmp(&self, other: &InsertOperation) -> Ordering {
//         match self.position.cmp(&other.position) {
//             Ordering::Equal => {
//                 self.state.site_id.cmp(&other.state.site_id)
//             },
//             x => x
//         }
//     }
// }

impl fmt::Debug for DeleteOperation {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "({}, {})[{}]", self.position, self.length, self.timestamp)
    }
}

// impl PartialOrd for DeleteOperation {
//     fn partial_cmp(&self, other: &DeleteOperation) -> Option<Ordering> {
//         match self.position.cmp(&other.position) {
//             Ordering::Equal => {
//                 self.state.site_id.partial_cmp(&other.state.site_id)
//             },
//             x => Some(x)
//         }
//     }
// }

// impl Ord for DeleteOperation {
//     fn cmp(&self, other: &DeleteOperation) -> Ordering {
//         match self.position.cmp(&other.position) {
//             Ordering::Equal => {
//                 self.state.site_id.cmp(&other.state.site_id)
//             },
//             x => x
//         }
//     }
// }
//
// impl PartialOrd for State {
//     fn partial_cmp(&self, other: &State) -> Option<Ordering> {
//         self.site_id.partial_cmp(&other.site_id)
//     }
// }
//
// impl Ord for State {
//     fn cmp(&self, other: &State) -> Ordering {
//         self.site_id.cmp(&other.site_id)
//     }
// }

// impl State {
//
//     /// Create a new state at the given site and give it the corresponding timestamps
//     #[inline]
//     pub fn new(site_id: u32, local_time: u32, remote_time: u32) -> State {
//         State {
//             site_id: site_id,
//             local_time: local_time,
//             remote_time: remote_time
//         }
//     }
//
//     /// Sets the local time to the given time stamp
//     pub fn set_time(&mut self, time_stamp: u32) {
//         self.local_time = time_stamp;
//     }
//
//     /// Gets the local timestamp for this state
//     pub fn get_time(&self) -> u32 {
//         self.local_time
//     }
//
//     /// Checks if this state (stored locally) happened at the same time
//     /// as another state (from a remote site).
//     pub fn matches(&self, other_state: &State) -> bool {
//         self.site_id == other_state.site_id && self.remote_time == other_state.remote_time
//     }
//
//     /// Checks if this state happened after a certain timestamp on a differenent site
//     pub fn happened_after(&self, other_time: u32) -> bool {
//         self.local_time > other_time
//     }
//
//     /// Gets the site id of the origin of this state
//     #[inline]
//     pub fn get_site_id(&self) -> u32 {
//         self.site_id
//     }
//
//     /// Compress this state and write to `writer`.  The output can then be expanded
//     /// back into an equivilent State using `expand_from()`
//     pub fn compress_to<W: Write>(&self, writer: &mut W) -> io::Result<()> {
//
//         let mut int_buf = [0;4];
//         NetworkEndian::write_u32(&mut int_buf, self.site_id);
//         try!(writer.write(&int_buf));
//         NetworkEndian::write_u32(&mut int_buf, self.local_time);
//         try!(writer.write(&int_buf));
//         NetworkEndian::write_u32(&mut int_buf, self.remote_time);
//         try!(writer.write(&int_buf));
//         Ok(())
//     }
//
//     /// Expand this engine from previously compressed data in `reader`.  The data in reader
//     /// should have been written using `compress_to()`
//     pub fn expand_from<R: Read>(reader: &mut R) -> io::Result<State> {
//         let mut int_buf = [0;4];
//         try!(reader.read_exact(&mut int_buf));
//         let site_id = NetworkEndian::read_u32(&int_buf);
//         try!(reader.read_exact(&mut int_buf));
//         let local_time = NetworkEndian::read_u32(&int_buf);
//         try!(reader.read_exact(&mut int_buf));
//         let remote_time = NetworkEndian::read_u32(&int_buf);
//         Ok(State {
//             site_id: site_id,
//             local_time: local_time,
//             remote_time: remote_time
//         })
//
//     }
//
// }

#[cfg(test)]
mod test {
    use super::{InsertOperation, DeleteOperation, OverlapResult, OperationInternal};

    #[test]
    fn overlapping() {
        // let state1 = State::new(1, 0, 0);
        // let state2 = State::new(2, 0, 0);

        // Insert / Insert
        let op1 = InsertOperation::new(2, "Some text".bytes().collect(), 0, 1);
        let op2 = InsertOperation::new(3, "Other words".bytes().collect(), 1, 2);
        assert_eq!(op1.check_overlap(&op2, 0, 1), OverlapResult::Precedes);

        let op1 = InsertOperation::new(2, "Some text".bytes().collect(), 0, 1);
        let op2 = InsertOperation::new(2, "Other words".bytes().collect(), 1, 2);
        assert_eq!(op1.check_overlap(&op2, 0, 1), OverlapResult::Follows);

        // Insert / Delete
        let op1 = InsertOperation::new(2, "Some text".bytes().collect(), 0, 1);
        let op2 = DeleteOperation::new(1, 5, 1);
        assert_eq!(op1.check_overlap(&op2, 0, 0), OverlapResult::EnclosedBy(1));

        let op1 = InsertOperation::new(2, "Some text".bytes().collect(), 0, 1);
        let op2 = DeleteOperation::new(1, 5, 1);
        assert_eq!(op1.check_overlap(&op2, -3, 0), OverlapResult::EnclosedBy(4));

        let op1 = InsertOperation::new(2, "Some text".bytes().collect(), 0, 1);
        let op2 = DeleteOperation::new(1, 5, 1);
        assert_eq!(op1.check_overlap(&op2, -4, 0), OverlapResult::Follows);

        let op1 = InsertOperation::new(2, "Some text".bytes().collect(), 0, 1);
        let op2 = DeleteOperation::new(1, 5, 1);
        assert_eq!(op1.check_overlap(&op2, 1, 0), OverlapResult::Precedes);

        // Delete / Insert
        let op1 = DeleteOperation::new(1, 5, 0);
        let op2 = InsertOperation::new(2, "Some text".bytes().collect(), 1, 2);
        assert_eq!(op1.check_overlap(&op2, 0, 1), OverlapResult::Follows);

        let op1 = DeleteOperation::new(1, 5, 0);
        let op2 = InsertOperation::new(2, "Some text".bytes().collect(), 1, 2);
        assert_eq!(op1.check_overlap(&op2, 0, -3), OverlapResult::Encloses(4));

        let op1 = DeleteOperation::new(1, 5, 0);
        let op2 = InsertOperation::new(2, "Some text".bytes().collect(), 1, 2);
        assert_eq!(op1.check_overlap(&op2, 0, -4), OverlapResult::Precedes);

        let op1 = DeleteOperation::new(11, 5, 0);
        let op2 = InsertOperation::new(2, "Some text".bytes().collect(), 1, 2);
        assert_eq!(op1.check_overlap(&op2, 0, 0), OverlapResult::Follows);

        // Delete / Delete
        let op1 = DeleteOperation::new(1, 5, 0);
        let op2 = DeleteOperation::new(6, 3, 1);
        assert_eq!(op1.check_overlap(&op2, 0, 0), OverlapResult::Precedes);

        let op1 = DeleteOperation::new(7, 1, 0);
        let op2 = DeleteOperation::new(4, 4, 1);
        assert_eq!(op1.check_overlap(&op2, 0, 1), OverlapResult::Follows);

        let op1 = DeleteOperation::new(1, 5, 0);
        let op2 = DeleteOperation::new(2, 4, 1);
        assert_eq!(op1.check_overlap(&op2, 0, 0), OverlapResult::OverlapFront(4));

        let op1 = DeleteOperation::new(1, 5, 0);
        let op2 = DeleteOperation::new(2, 3, 1);
        assert_eq!(op1.check_overlap(&op2, 0, 0), OverlapResult::Encloses(1));

        let op1 = DeleteOperation::new(1, 5, 0);
        let op2 = DeleteOperation::new(1, 5, 1);
        assert_eq!(op1.check_overlap(&op2, 0, 0), OverlapResult::OverlapBack(5));

        let op1 = DeleteOperation::new(1, 5, 0);
        let op2 = DeleteOperation::new(0, 4, 1);
        assert_eq!(op1.check_overlap(&op2, 0, -1), OverlapResult::OverlapBack(4));

        let op1 = DeleteOperation::new(4, 2, 0);
        let op2 = DeleteOperation::new(3, 2, 1);
        assert_eq!(op1.check_overlap(&op2, 0, 0), OverlapResult::OverlapBack(1));

        let op1 = DeleteOperation::new(4, 2, 0);
        let op2 = DeleteOperation::new(3, 3, 1);
        assert_eq!(op1.check_overlap(&op2, 0, 0), OverlapResult::OverlapBack(2));

        let op1 = DeleteOperation::new(4, 2, 0);
        let op2 = DeleteOperation::new(3, 4, 1);
        assert_eq!(op1.check_overlap(&op2, 0, 0), OverlapResult::EnclosedBy(1));

        let op1 = DeleteOperation::new(9, 4, 0);
        let op2 = DeleteOperation::new(2, 2, 1);
        assert_eq!(op1.check_overlap(&op2, 0, -5), OverlapResult::Follows);

    }
}
