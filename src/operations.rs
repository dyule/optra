use std::cmp::Ordering;
use std::fmt::Debug;
use ::{Offset, Position, OverlapResult};

pub trait Operation: Debug {
    fn get_state(&self) -> &State;
    fn get_state_mut(&mut self) -> &mut State;
    fn compare_with_offsets<O: Operation>(&self, other: &O, my_offset: Offset, other_offset: Offset) -> bool {
        let my_pos = self.get_position() as Offset - my_offset;
        let other_pos = other.get_position() as Offset - other_offset;
        my_pos < other_pos || my_pos == other_pos && self.get_state().site_id < other.get_state().site_id
    }

    fn compare_with_offsets_no_tiebreak<O: Operation>(&self, other: &O, my_offset: Offset, other_offset: Offset) -> bool {
        let my_pos = self.get_position() as Offset - my_offset;
        let other_pos = other.get_position() as Offset - other_offset;
        my_pos < other_pos
    }

    fn get_position(&self) -> Position;
    fn get_increment(&self) -> Offset;
    fn update_position_by(&mut self, delta: Offset);
    fn resolve_overlap<O: Operation>(&self, other: &O, my_offset: Offset, other_offset: Offset) -> OverlapResult;
    fn resolve_overlap_with_insert(&self, other: &InsertOperation, my_offset: Offset, other_offset: Offset) -> OverlapResult;
    fn resolve_overlap_with_delete(&self, other: &DeleteOperation, my_offset: Offset, other_offset: Offset) -> OverlapResult;

}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct InsertOperation {
    state: State,
    position: Position,
    value:Vec<u8>
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct DeleteOperation{
    state: State,
    position: Position,
    length: Position
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct State {
    pub site_id: u32,
    local_time: u32,
    remote_time: u32
}

impl InsertOperation {
    #[inline]
    pub fn new(position: Position, value: Vec<u8>, state: State) -> InsertOperation {
        InsertOperation {
            position: position,
            value: value,
            state: state
        }
    }

    pub fn get_value(&self) -> &[u8] {
        &self.value
    }
}

impl DeleteOperation {
    #[inline]
    pub fn new(position: Position, length: Position, state: State) -> DeleteOperation {
        DeleteOperation {
            position: position,
            length: length,
            state: state
        }
    }

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

    fn update_position_by(&mut self, delta: Offset) {
        self.position = (self.position as Offset +  delta) as Position
    }

    #[inline]
    fn resolve_overlap<O: Operation>(&self,  other: &O, my_offset: Offset, other_offset: Offset) -> OverlapResult {
        other.resolve_overlap_with_insert(self, other_offset, my_offset)
    }
    #[inline]
    fn resolve_overlap_with_insert(&self, other: &InsertOperation, my_offset: Offset, other_offset: Offset) -> OverlapResult {
        if other.compare_with_offsets(self, other_offset, my_offset) {
            OverlapResult::Precedes
        } else {
            OverlapResult::Follows
        }
    }

    fn resolve_overlap_with_delete(&self, other: &DeleteOperation, my_offset: Offset, other_offset: Offset) -> OverlapResult {
        if other.compare_with_offsets(self, other_offset, my_offset) {
            if self.compare_with_offsets_no_tiebreak(other, my_offset, other_offset - other.length as Offset) {
                OverlapResult::Encloses
            } else {
                OverlapResult::Precedes
            }
        } else {
            OverlapResult::Follows
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

    fn update_position_by(&mut self, delta: Offset) {
        self.position = (self.position as Offset +  delta) as Position
    }

    #[inline]
    fn resolve_overlap<O: Operation>(&self,  other: &O, my_offset: Offset, other_offset: Offset) -> OverlapResult {
        other.resolve_overlap_with_delete(self, other_offset, my_offset)
    }


    fn resolve_overlap_with_insert(&self, other: &InsertOperation, my_offset: Offset, other_offset: Offset) -> OverlapResult {
        if other.compare_with_offsets(self, other_offset, my_offset) {
            OverlapResult::Precedes
        } else {
            if other.compare_with_offsets_no_tiebreak(self, other_offset, my_offset - self.length as Offset) {
                OverlapResult::EnclosedBy
            } else {
                OverlapResult::Follows
            }
        }
    }
    fn resolve_overlap_with_delete(&self, other: &DeleteOperation, my_offset: Offset, other_offset: Offset) -> OverlapResult {
        // |--other--
        //     |--self--
        if other.compare_with_offsets(self, other_offset, my_offset) {
            // |--other--|
            //    |--self--
            if self.compare_with_offsets_no_tiebreak(other, my_offset, other_offset - other.length as Offset) {
                // |--other-----|
                //    |--self--|
                if self.compare_with_offsets_no_tiebreak(other, my_offset - self.length as Offset, other_offset - other.length as Offset) {
                    OverlapResult::Encloses
                }
                // |--other--|
                //    |--self--|
                else {
                    OverlapResult::OverlapFront
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
            if other.compare_with_offsets_no_tiebreak(self, other_offset, my_offset - self.length as Offset) {
                //    |--other--|
                // |--self-------|
                if other.compare_with_offsets_no_tiebreak(self,other_offset - other.length as Offset, my_offset - self.length as Offset) {
                    OverlapResult::EnclosedBy
                }
                //    |--other--|
                // |--self--|
                else {
                    OverlapResult::OverlapBack
                }
            }
            //            |--other--
            // |--self--|
            else {
                OverlapResult::Follows
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

    pub fn get_time(&self) -> u32 {
        self.local_time
    }

    /// Checks if this state (stored locally) happened at the same time
    /// as another state (from a remote site).
    pub fn matches(&self, other_state: &State) -> bool {
        self.site_id == other_state.site_id && self.remote_time == other_state.remote_time
    }

    /// Checks if this state happened after a certain timestamp
    pub fn happened_after(&self, other_time: u32) -> bool {
        self.local_time > other_time
    }

}

#[cfg(test)]
mod test {
    use super::{InsertOperation, DeleteOperation, Operation, State};
    use OverlapResult;
    #[test]
    fn overlapping() {
        let state1 = State::new(1, 0, 0);
        let state2 = State::new(2, 0, 0);

        // Insert / Insert
        let op1 = InsertOperation::new(2, "Some text".bytes().collect(), state1.clone());
        let op2 = InsertOperation::new(3, "Other words".bytes().collect(), state2.clone());
        assert_eq!(op1.resolve_overlap(&op2, 0, 1), OverlapResult::Precedes);

        let op1 = InsertOperation::new(2, "Some text".bytes().collect(), state1.clone());
        let op2 = InsertOperation::new(2, "Other words".bytes().collect(), state2.clone());
        assert_eq!(op1.resolve_overlap(&op2, 0, 1), OverlapResult::Follows);

        // Insert / Delete
        let op1 = InsertOperation::new(2, "Some text".bytes().collect(), state1.clone());
        let op2 = DeleteOperation::new(1, 5, state2.clone());
        assert_eq!(op1.resolve_overlap(&op2, 0, 0), OverlapResult::EnclosedBy);

        let op1 = InsertOperation::new(2, "Some text".bytes().collect(), state1.clone());
        let op2 = DeleteOperation::new(1, 5, state2.clone());
        assert_eq!(op1.resolve_overlap(&op2, -3, 0), OverlapResult::EnclosedBy);

        let op1 = InsertOperation::new(2, "Some text".bytes().collect(), state1.clone());
        let op2 = DeleteOperation::new(1, 5, state2.clone());
        assert_eq!(op1.resolve_overlap(&op2, -4, 0), OverlapResult::Follows);

        let op1 = InsertOperation::new(2, "Some text".bytes().collect(), state1.clone());
        let op2 = DeleteOperation::new(1, 5, state2.clone());
        assert_eq!(op1.resolve_overlap(&op2, 1, 0), OverlapResult::Precedes);

        // Delete / Insert
        let op1 = DeleteOperation::new(1, 5, state1.clone());
        let op2 = InsertOperation::new(2, "Some text".bytes().collect(), state2.clone());
        assert_eq!(op1.resolve_overlap(&op2, 0, 1), OverlapResult::Encloses);

        let op1 = DeleteOperation::new(1, 5, state1.clone());
        let op2 = InsertOperation::new(2, "Some text".bytes().collect(), state2.clone());
        assert_eq!(op1.resolve_overlap(&op2, 0, -3), OverlapResult::Encloses);

        let op1 = DeleteOperation::new(1, 5, state1.clone());
        let op2 = InsertOperation::new(2, "Some text".bytes().collect(), state2.clone());
        assert_eq!(op1.resolve_overlap(&op2, 0, -4), OverlapResult::Precedes);

        let op1 = DeleteOperation::new(11, 5, state1.clone());
        let op2 = InsertOperation::new(2, "Some text".bytes().collect(), state2.clone());
        assert_eq!(op1.resolve_overlap(&op2, 0, 0), OverlapResult::Follows);

        // Delete / Delete
        let op1 = DeleteOperation::new(1, 5, state1.clone());
        let op2 = DeleteOperation::new(6, 3, state2.clone());
        assert_eq!(op1.resolve_overlap(&op2, 0, 0), OverlapResult::Precedes);

        let op1 = DeleteOperation::new(7, 1, state1.clone());
        let op2 = DeleteOperation::new(4, 4, state2.clone());
        assert_eq!(op1.resolve_overlap(&op2, 0, 1), OverlapResult::Follows);

        let op1 = DeleteOperation::new(1, 5, state1.clone());
        let op2 = DeleteOperation::new(2, 4, state2.clone());
        assert_eq!(op1.resolve_overlap(&op2, 0, 0), OverlapResult::OverlapFront);

        let op1 = DeleteOperation::new(1, 5, state1.clone());
        let op2 = DeleteOperation::new(2, 3, state2.clone());
        assert_eq!(op1.resolve_overlap(&op2, 0, 0), OverlapResult::Encloses);

        let op1 = DeleteOperation::new(1, 5, state1.clone());
        let op2 = DeleteOperation::new(1, 5, state2.clone());
        assert_eq!(op1.resolve_overlap(&op2, 0, 0), OverlapResult::OverlapFront);

        let op1 = DeleteOperation::new(1, 5, state1.clone());
        let op2 = DeleteOperation::new(0, 4, state2.clone());
        assert_eq!(op1.resolve_overlap(&op2, 0, -1), OverlapResult::Encloses);

        let op1 = DeleteOperation::new(4, 2, state1.clone());
        let op2 = DeleteOperation::new(3, 2, state2.clone());
        assert_eq!(op1.resolve_overlap(&op2, 0, 0), OverlapResult::OverlapBack);

        let op1 = DeleteOperation::new(4, 2, state1.clone());
        let op2 = DeleteOperation::new(3, 3, state2.clone());
        assert_eq!(op1.resolve_overlap(&op2, 0, 0), OverlapResult::OverlapBack);

        let op1 = DeleteOperation::new(4, 2, state1.clone());
        let op2 = DeleteOperation::new(3, 4, state2.clone());
        assert_eq!(op1.resolve_overlap(&op2, 0, 0), OverlapResult::EnclosedBy);
    }
}
