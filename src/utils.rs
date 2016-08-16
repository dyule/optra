use super::operations::{Operation, DeleteOperation, OverlapResult, CrossResult, OperationInternal, Advance};
use Offset;

pub struct SequenceSwapper {
    incoming_offset: Offset,
    existing_offset: Offset,
}


pub struct SequenceTransformer {
    incoming_offset: Offset,
    existing_offset: Offset,
    total_overlap: Offset,
}

impl SequenceTransformer {
    #[inline]
    pub fn new() -> SequenceTransformer {
        SequenceTransformer {
            incoming_offset: 0,
            existing_offset: 0,
            total_overlap: 0,
        }
    }

    pub fn transform_operations<O1: OperationInternal, O2: OperationInternal>(&mut self, incoming_operation: &mut O1, exisiting_operation: &O2) -> Advance<O1> {
        trace!("Before: Existing: {:?}, Offset: {:?}. Incoming: {:?}, Offset: {:?}, overlap: {}", exisiting_operation, self.existing_offset, incoming_operation, self.incoming_offset, self.total_overlap);
        let overlap_result = incoming_operation.check_overlap(exisiting_operation, self.incoming_offset, self.existing_offset);
        let r = self.update_with(overlap_result, incoming_operation, exisiting_operation);
        trace!("After: Existing: {:?}, Offset: {:?}. Incoming: {:?}, Offset: {:?}, overlap: {}", exisiting_operation, self.existing_offset, incoming_operation, self.incoming_offset, self.total_overlap);
        r
    }

    pub fn transform_single<O: OperationInternal>(&self, operation: &mut O) {
        operation.update_position_by(self.existing_offset + self.total_overlap);
    }

    fn update_with<O1: OperationInternal, O2: OperationInternal>(&mut self, overlap: OverlapResult, incoming_operation: &mut O1, exisiting_operation: &O2) -> Advance<O1> {
        trace!("Overlap: {:?}", overlap);
        match overlap {
            OverlapResult::Precedes => {
                self.incoming_offset += incoming_operation.get_increment();
                incoming_operation.update_position_by(self.existing_offset + self.total_overlap);
                Advance::Incoming
            },
            OverlapResult::Follows => {
                self.existing_offset += exisiting_operation.get_increment();
                Advance::Existing
            },
            OverlapResult::EnclosedBy(front_difference) => {
                self.incoming_offset += incoming_operation.get_increment();
                //move to front of the other operation
                incoming_operation.update_position_by(self.existing_offset + self.total_overlap - front_difference as Offset);
                self.total_overlap -= incoming_operation.get_increment() as Offset;
                // remove its length
                incoming_operation.set_length_to_zero();
                Advance::Incoming
            },
            OverlapResult::Encloses(front_difference) => {
                let new_op = incoming_operation.split(front_difference);
                self.incoming_offset += incoming_operation.get_increment();
                incoming_operation.update_position_by(self.existing_offset + self.total_overlap);
                Advance::Neither(new_op)
            },
            OverlapResult::OverlapBack(amount) => {
                self.existing_offset += exisiting_operation.get_increment();
                self.total_overlap += amount as Offset;
                self.incoming_offset -= amount as Offset;
                incoming_operation.update_size_by(-(amount as Offset));
                //incoming_operation.update_position_by(amount as Offset);

                Advance::Existing
            },
            OverlapResult::OverlapFront(amount) => {
                self.incoming_offset += incoming_operation.get_increment();

                incoming_operation.update_size_by(-(amount as Offset));
                incoming_operation.update_position_by(self.existing_offset + self.total_overlap);
                self.total_overlap += amount as Offset;
                Advance::Incoming
            },
        }
    }
}

impl SequenceSwapper {
    #[inline]
    pub fn new() -> SequenceSwapper {
        SequenceSwapper {
            incoming_offset: 0,
            existing_offset: 0,
        }
    }

    pub fn swap_operations<O: OperationInternal>(&mut self, incoming_operation: &mut O, exisiting_operation: &mut DeleteOperation) -> Advance<O> {
        trace!("Before: Existing: {:?}, Offset: {:?}. Incoming: {:?}, Offset: {:?}", exisiting_operation, self.existing_offset, incoming_operation, self.incoming_offset);
        let overlap_result = exisiting_operation.crossed_by(incoming_operation, self.existing_offset, self.incoming_offset + self.existing_offset);
        trace!("Cross: {:?}", overlap_result);
        let r = match overlap_result {
            CrossResult::Precedes => {
                self.incoming_offset += incoming_operation.get_increment();
                incoming_operation.update_position_by(-self.existing_offset);
                Advance::Incoming
            },
            CrossResult::Follows => {
                self.existing_offset += exisiting_operation.get_increment();
                exisiting_operation.update_position_by(self.incoming_offset);
                Advance::Existing
            },
            CrossResult::Crosses(front_difference) => {
                let new_op = incoming_operation.split(front_difference);
                self.incoming_offset += incoming_operation.get_increment();
                incoming_operation.update_position_by(-self.existing_offset);
                Advance::Neither(new_op)
            }
        };
        trace!("After: Existing: {:?}, Offset: {:?}. Incoming: {:?}, Offset: {:?}", exisiting_operation, self.existing_offset, incoming_operation, self.incoming_offset);
        r
    }

    pub fn swap_single<O: OperationInternal>(&self, operation: &mut O) {
        operation.update_position_by(-self.existing_offset);
    }

    pub fn swap_existing(&self, operation: &mut DeleteOperation) {
        operation.update_position_by(self.incoming_offset);
    }
}
