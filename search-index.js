var searchIndex = {};
searchIndex["optra"] = {"doc":"An engine for keeping remote siles synchronized.","items":[[3,"InsertOperation","optra","Represents an operation which inserts data into a file",null,null],[3,"DeleteOperation","","Represents an operation which removes data from a file",null,null],[3,"State","","Represents the state of a document.  Essentially a timestamp and a site id.",null,null],[3,"Engine","","Process file change operations in such a way that they can be synchronized across sites",null,null],[3,"TransactionSequence","","Represents a sequence of transactions that can be performed on a file.",null,null],[3,"OTError","","Represents an error in attempting to synchronize remote operations",null,null],[12,"kind","","The kind of error this is",0,null],[4,"ErrorKind","","Represents the kind of error we encountered synchronizing operations",null,null],[13,"NoSuchState","","The remote operations refer to a state that we have not yet recieved",1,null],[11,"clone","","",2,null],[11,"eq","","",2,null],[11,"ne","","",2,null],[11,"clone","","",3,null],[11,"eq","","",3,null],[11,"ne","","",3,null],[11,"clone","","",4,null],[11,"eq","","",4,null],[11,"ne","","",4,null],[11,"fmt","","",4,null],[11,"new","","Creates a new `InsertOperation` that will insert the bytes represented by `value` in a file at location `position`",2,{"inputs":[{"name":"u64"},{"name":"vec"},{"name":"state"}],"output":{"name":"insertoperation"}}],[11,"get_value","","Gets the bytes that will be inserted when this operation is applied",2,null],[11,"new","","Creates a new `DeleteOperation` that woll delete `length` bytes at `position` in a file",3,{"inputs":[{"name":"u64"},{"name":"u64"},{"name":"state"}],"output":{"name":"deleteoperation"}}],[11,"get_length","","Gets the number of bytes that will be removed when the delete operation is applied",3,null],[11,"get_state","","",2,null],[11,"get_state_mut","","",2,null],[11,"get_position","","",2,null],[11,"get_increment","","",2,null],[11,"get_state","","",3,null],[11,"get_state_mut","","",3,null],[11,"get_position","","",3,null],[11,"get_increment","","",3,null],[11,"partial_cmp","","",2,null],[11,"fmt","","",2,null],[11,"cmp","","",2,null],[11,"fmt","","",3,null],[11,"partial_cmp","","",3,null],[11,"cmp","","",3,null],[11,"partial_cmp","","",4,null],[11,"cmp","","",4,null],[11,"new","","Create a new state at the given site and give it the corresponding timestamps",4,{"inputs":[{"name":"u32"},{"name":"u32"},{"name":"u32"}],"output":{"name":"state"}}],[11,"set_time","","Sets the local time to the given time stamp",4,null],[11,"get_time","","Gets the local timestamp for this state",4,null],[11,"matches","","Checks if this state (stored locally) happened at the same time\nas another state (from a remote site).",4,null],[11,"happened_after","","Checks if this state happened after a certain timestamp on a differenent site",4,null],[11,"get_site_id","","Gets the site id of the origin of this state",4,null],[11,"fmt","","",5,null],[11,"new","","Creates a new engine for the given site id.  The id should be\nunique across all clients, and probably generated by the server",6,{"inputs":[{"name":"u32"}],"output":{"name":"engine"}}],[11,"process_diffs","","Convert the diffs we got from analyzing a file into a TransactionSequence\nwe can send to another site for synchronization.",6,null],[11,"integrate_remote","","Integrates the sequence of operations given by `remote_sequence` into the local history.  The ordering\nproperties of the local history will be maintained, and a sequence of operations that\ncan be applied to the local state will be returned.",6,null],[11,"process_transaction","","Processes a series of operations prior to being sent out to remote sites.  The operations must\nhave been performed on the data after every operation in the local history, but no others.  The\noperations in the transaction must also be effect order, with the inserts preceding the deletes.",6,null],[11,"new","","Construct a new `TransactionSequence` from the given operations and metadata\nThe `starting_state` is the state of the file (as represented by a time_stamp) at the time the sequence was created, or `None` if the file was newly created.\nThe site is the location this `TransactionSequence` came from.",5,{"inputs":[{"name":"option"},{"name":"u32"},{"name":"linkedlist"},{"name":"linkedlist"}],"output":{"name":"transactionsequence"}}],[11,"apply","","Apply the operations in this sequence to a file.  This should not be called until after\nthe sequence has been integrated via [`Engine::integrate_remote`](struct.Engine.html#method.integrate_remote)",5,null],[8,"Operation","","An operation that will make a change to a file.",null,null],[10,"get_state","","Gets the [`State`](struct.State.html) that this operation was performed on",7,null],[10,"get_state_mut","","Gets the [`State`](struct.State.html) that this operation was performed on mutably.",7,null],[10,"get_position","","Gets the position this operation will be perfomed at",7,null],[10,"get_increment","","Gets the size change this operation will perform.  For insert operations, it&#39;s the\nlength of the data they will insert.  For delete operations, it&#39;s the length\nof the data they will delete",7,null],[11,"fmt","","",0,null],[11,"fmt","","",1,null],[11,"new","","Create a new OTError of the given kind.",0,{"inputs":[{"name":"errorkind"}],"output":{"name":"oterror"}}]],"paths":[[3,"OTError"],[4,"ErrorKind"],[3,"InsertOperation"],[3,"DeleteOperation"],[3,"State"],[3,"TransactionSequence"],[3,"Engine"],[8,"Operation"]]};
initSearch(searchIndex);