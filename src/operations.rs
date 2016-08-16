use std::cmp::Ordering;
use std::fmt;
use ::{Offset, Position};

/// An operation that will make a change to a file.
pub trait Operation: fmt::Debug + Ord + Clone {
    /// Gets the [`State`](struct.State.html) that this operation was performed on
    fn get_state(&self) -> &State;
    /// Gets the [`State`](struct.State.html) that this operation was performed on mutably.
    fn get_state_mut(&mut self) -> &mut State;

    /// Gets the position this operation will be perfomed at
    fn get_position(&self) -> Position;

    /// Gets the size change this operation will perform.  For insert operations, it's the
    /// length of the data they will insert.  For delete operations, it's the length
    /// of the data they will delete
    fn get_increment(&self) -> Offset;

}

/// Represents an operation which inserts data into a file
#[derive(Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct InsertOperation {
    state: State,
    position: Position,
    value:Vec<u8>
}

/// Represents an operation which removes data from a file
#[derive(Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct DeleteOperation{
    state: State,
    position: Position,
    length: Position
}

/// Represents the state of a document.  Essentially a timestamp and a site id.
///
/// The state has two timestamps, the remtoe timestamp (the stamp given to it by the site that originated it)
/// and its local timestamp (which is the timestamp this site gave to it)
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct State {
    site_id: u32,
    local_time: u32,
    remote_time: u32
}

pub trait OperationInternal: Operation {
    fn compare_with_offsets<O: Operation>(&self, other: &O, my_offset: Offset, other_offset: Offset) -> bool {
        let my_pos = self.get_position() as Offset - my_offset;
        let other_pos = other.get_position() as Offset - other_offset;
        my_pos < other_pos || my_pos == other_pos && self.get_state().get_site_id() < other.get_state().get_site_id()
    }

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
    pub fn new(position: Position, value: Vec<u8>, state: State) -> InsertOperation {
        InsertOperation {
            position: position,
            value: value,
            state: state
        }
    }

    /// Gets the bytes that will be inserted when this operation is applied
    pub fn get_value(&self) -> &[u8] {
        &self.value
    }
}


impl DeleteOperation {

    /// Creates a new `DeleteOperation` that woll delete `length` bytes at `position` in a file
    #[inline]
    pub fn new(position: Position, length: Position, state: State) -> DeleteOperation {
        DeleteOperation {
            position: position,
            length: length,
            state: state
        }
    }

    /// Gets the number of bytes that will be removed when the delete operation is applied
    pub fn get_length(&self) -> Position {
        self.length
    }
}

impl Operation for InsertOperation {
    #[inline]
    fn get_state(&self) -> & State {
        &self.state
    }

    #[inline]
    fn get_state_mut(&mut self) -> &mut State {
        &mut self.state
    }

    #[inline]
    fn get_position(&self) -> Position {
        self.position
    }

    #[inline]
    fn get_increment(&self) -> Offset {
        self.value.len() as Offset
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
    #[inline]
    fn get_state(&self) -> & State {
        &self.state
    }

    #[inline]
    fn get_state_mut(&mut self) -> &mut State {
        &mut self.state
    }

    #[inline]
    fn get_position(&self) -> Position {
        self.position
    }

    #[inline]
    fn get_increment(&self) -> Offset {
        -(self.length as Offset)
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
        let new_op = DeleteOperation::new(self.position , self.length - split_pos, self.state.clone());
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

impl PartialOrd for InsertOperation {
    fn partial_cmp(&self, other: &InsertOperation) -> Option<Ordering> {
        match self.position.cmp(&other.position) {
            Ordering::Equal => {
                self.state.site_id.partial_cmp(&other.state.site_id)
            },
            x => Some(x)
        }
    }
}

impl fmt::Debug for InsertOperation {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "({}, {})", self.position, String::from_utf8_lossy(&self.value))
    }
}

impl Ord for InsertOperation {
    fn cmp(&self, other: &InsertOperation) -> Ordering {
        match self.position.cmp(&other.position) {
            Ordering::Equal => {
                self.state.site_id.cmp(&other.state.site_id)
            },
            x => x
        }
    }
}

impl fmt::Debug for DeleteOperation {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "({}, {})", self.position, self.length)
    }
}

impl PartialOrd for DeleteOperation {
    fn partial_cmp(&self, other: &DeleteOperation) -> Option<Ordering> {
        match self.position.cmp(&other.position) {
            Ordering::Equal => {
                self.state.site_id.partial_cmp(&other.state.site_id)
            },
            x => Some(x)
        }
    }
}

impl Ord for DeleteOperation {
    fn cmp(&self, other: &DeleteOperation) -> Ordering {
        match self.position.cmp(&other.position) {
            Ordering::Equal => {
                self.state.site_id.cmp(&other.state.site_id)
            },
            x => x
        }
    }
}

impl PartialOrd for State {
    fn partial_cmp(&self, other: &State) -> Option<Ordering> {
        self.site_id.partial_cmp(&other.site_id)
    }
}

impl Ord for State {
    fn cmp(&self, other: &State) -> Ordering {
        self.site_id.cmp(&other.site_id)
    }
}

impl State {

    /// Create a new state at the given site and give it the corresponding timestamps
    #[inline]
    pub fn new(site_id: u32, local_time: u32, remote_time: u32) -> State {
        State {
            site_id: site_id,
            local_time: local_time,
            remote_time: remote_time
        }
    }

    /// Sets the local time to the given time stamp
    pub fn set_time(&mut self, time_stamp: u32) {
        self.local_time = time_stamp;
    }

    /// Gets the local timestamp for this state
    pub fn get_time(&self) -> u32 {
        self.local_time
    }

    /// Checks if this state (stored locally) happened at the same time
    /// as another state (from a remote site).
    pub fn matches(&self, other_state: &State) -> bool {
        self.site_id == other_state.site_id && self.remote_time == other_state.remote_time
    }

    /// Checks if this state happened after a certain timestamp on a differenent site
    pub fn happened_after(&self, other_time: u32, other_id: u32) -> bool {
        self.local_time > other_time && self.site_id != other_id
    }

    /// Gets the site id of the origin of this state
    #[inline]
    pub fn get_site_id(&self) -> u32 {
        self.site_id
    }

}

#[cfg(test)]
mod test {
    use super::{InsertOperation, DeleteOperation, State, OverlapResult, OperationInternal};

    #[test]
    fn overlapping() {
        let state1 = State::new(1, 0, 0);
        let state2 = State::new(2, 0, 0);

        // Insert / Insert
        let op1 = InsertOperation::new(2, "Some text".bytes().collect(), state1.clone());
        let op2 = InsertOperation::new(3, "Other words".bytes().collect(), state2.clone());
        assert_eq!(op1.check_overlap(&op2, 0, 1), OverlapResult::Precedes);

        let op1 = InsertOperation::new(2, "Some text".bytes().collect(), state1.clone());
        let op2 = InsertOperation::new(2, "Other words".bytes().collect(), state2.clone());
        assert_eq!(op1.check_overlap(&op2, 0, 1), OverlapResult::Follows);

        // Insert / Delete
        let op1 = InsertOperation::new(2, "Some text".bytes().collect(), state1.clone());
        let op2 = DeleteOperation::new(1, 5, state2.clone());
        assert_eq!(op1.check_overlap(&op2, 0, 0), OverlapResult::EnclosedBy(1));

        let op1 = InsertOperation::new(2, "Some text".bytes().collect(), state1.clone());
        let op2 = DeleteOperation::new(1, 5, state2.clone());
        assert_eq!(op1.check_overlap(&op2, -3, 0), OverlapResult::EnclosedBy(4));

        let op1 = InsertOperation::new(2, "Some text".bytes().collect(), state1.clone());
        let op2 = DeleteOperation::new(1, 5, state2.clone());
        assert_eq!(op1.check_overlap(&op2, -4, 0), OverlapResult::Follows);

        let op1 = InsertOperation::new(2, "Some text".bytes().collect(), state1.clone());
        let op2 = DeleteOperation::new(1, 5, state2.clone());
        assert_eq!(op1.check_overlap(&op2, 1, 0), OverlapResult::Precedes);

        // Delete / Insert
        let op1 = DeleteOperation::new(1, 5, state1.clone());
        let op2 = InsertOperation::new(2, "Some text".bytes().collect(), state2.clone());
        assert_eq!(op1.check_overlap(&op2, 0, 1), OverlapResult::Follows);

        let op1 = DeleteOperation::new(1, 5, state1.clone());
        let op2 = InsertOperation::new(2, "Some text".bytes().collect(), state2.clone());
        assert_eq!(op1.check_overlap(&op2, 0, -3), OverlapResult::Encloses(4));

        let op1 = DeleteOperation::new(1, 5, state1.clone());
        let op2 = InsertOperation::new(2, "Some text".bytes().collect(), state2.clone());
        assert_eq!(op1.check_overlap(&op2, 0, -4), OverlapResult::Precedes);

        let op1 = DeleteOperation::new(11, 5, state1.clone());
        let op2 = InsertOperation::new(2, "Some text".bytes().collect(), state2.clone());
        assert_eq!(op1.check_overlap(&op2, 0, 0), OverlapResult::Follows);

        // Delete / Delete
        let op1 = DeleteOperation::new(1, 5, state1.clone());
        let op2 = DeleteOperation::new(6, 3, state2.clone());
        assert_eq!(op1.check_overlap(&op2, 0, 0), OverlapResult::Precedes);

        let op1 = DeleteOperation::new(7, 1, state1.clone());
        let op2 = DeleteOperation::new(4, 4, state2.clone());
        assert_eq!(op1.check_overlap(&op2, 0, 1), OverlapResult::Follows);

        let op1 = DeleteOperation::new(1, 5, state1.clone());
        let op2 = DeleteOperation::new(2, 4, state2.clone());
        assert_eq!(op1.check_overlap(&op2, 0, 0), OverlapResult::OverlapFront(4));

        let op1 = DeleteOperation::new(1, 5, state1.clone());
        let op2 = DeleteOperation::new(2, 3, state2.clone());
        assert_eq!(op1.check_overlap(&op2, 0, 0), OverlapResult::Encloses(1));

        let op1 = DeleteOperation::new(1, 5, state1.clone());
        let op2 = DeleteOperation::new(1, 5, state2.clone());
        assert_eq!(op1.check_overlap(&op2, 0, 0), OverlapResult::OverlapBack(5));

        let op1 = DeleteOperation::new(1, 5, state1.clone());
        let op2 = DeleteOperation::new(0, 4, state2.clone());
        assert_eq!(op1.check_overlap(&op2, 0, -1), OverlapResult::OverlapBack(4));

        let op1 = DeleteOperation::new(4, 2, state1.clone());
        let op2 = DeleteOperation::new(3, 2, state2.clone());
        assert_eq!(op1.check_overlap(&op2, 0, 0), OverlapResult::OverlapBack(1));

        let op1 = DeleteOperation::new(4, 2, state1.clone());
        let op2 = DeleteOperation::new(3, 3, state2.clone());
        assert_eq!(op1.check_overlap(&op2, 0, 0), OverlapResult::OverlapBack(2));

        let op1 = DeleteOperation::new(4, 2, state1.clone());
        let op2 = DeleteOperation::new(3, 4, state2.clone());
        assert_eq!(op1.check_overlap(&op2, 0, 0), OverlapResult::EnclosedBy(1));

        let op1 = DeleteOperation::new(9, 4, state1.clone());
        let op2 = DeleteOperation::new(2, 2, state2.clone());
        assert_eq!(op1.check_overlap(&op2, 0, -5), OverlapResult::Follows);

    }
}
