# Specification from the Embedded-HAL docs
Note: this is still a work in progress

## 1. Digital

### 1. OutputPin
Preconditions:
- Pin is configured as push-pull 
- connected to a high-impedance load

Requirement 1: set_low
After a call to set_low the pin is driven to ground

Requirement 2: set_high
After a call to set_high the pin is driven to VDD

Requirement 3: set_state
Should set the pin either high or low depending on input and matching the behaviour of set_low and set_high.

Requirement 4: idempotence
independence from previous state
The calls to set_low,set_high or set_state should not depend on the current state of the pin and previous calls to these methods.

Requirement 5: error
No calls may error in this test set-up

Requirement 6: State persistence
The pin should maintain its set state until explicitly changed by another call.



### 2. StatefulOutputPin
Preconditions are the same as OutputPin, additionally all requirements of OutputPin should hold for StatefulOutputPin as it is a super trait.

Requirement 1: is_set_high
Independent of the current state, is_set_high should return true if the pin is currently being driven high

Requirement 2: is_set_low
Independent of the current state, is_set_low should return true if the pin is currently being driven low

Requirement 3: toggle
After a call to toggle the pin should be driven to the logical opposite of its current state.
The resulting state should match the behaviour defined in OutputPin requirements 1 and 2

Requirement 4: idempotence
The calls to is_set_high, is_set_low, should not depend on the current state of the pin and previous calls to these methods.

Requirement 5: error
No calls may error in this test set-up

Requirement 6: Consistency
is_set_high and is_set_low should never return the same value

### 3. InputPin
Preconditions:
- Pin is configured as high-impedance input
- no pull-up or pull-down resistors
- the pin will be driven high or low by an external circuit

Requirement 1: is_high
Should return true if the pin is being driven high, false otherwise


Requirement 2: is_low
Should return true if the pin is being driven low, false otherwise

Requirement 3: idempotence
Calls to is_high or is_low should not affect future calls

Requirement 4: error
No calls may error in this test set-up

Requirement 5: mutual exclusivity
is_high() and is_low() should return opposite values

### 4. Wait (on Pin)
note: from embedded-hal-async

Preconditions:
- Pin is configured as input (just like 3. InputPin)
- Async runtime/executor is available
- Pin state can change during wait period

Requirement 1: wait_for_high
Should complete when the pin reads high, matching InputPin::is_high() behavior

Requirement 2: wait_for_low
Should complete when the pin reads low, matching InputPin::is_low() behavior

Requirement 3: wait_for_rising_edge
Should complete on low-to-high transition, requiring the pin to be low then become high

Requirement 4: wait_for_falling_edge
Should complete on high-to-low transition, requiring the pin to be high then become low

Requirement 5: wait_for_any_edge
Should complete on any state transition (either rising or falling edge)

Requirement 6: waker behavior
Must properly wake the async task when the waited-for condition is met

Requirement 7: no spurious wakeups
Should not complete unless the specified condition has actually occurred

Requirement 8: cancellation safety
Wait operations should be safely cancellable without leaving the pin in an undefined state%

## 2. I2C 
Preconditions:
- I2C bus is properly configured with appropriate pull-up resistors
- Single master configuration

Requirement 1: no bus
The implementation should not implement higher abstractions like a bus and simply represent the underlying interface

Requirement 2: no pipelining
Operations should complete fully before returning
https://docs.rs/embedded-hal/latest/embedded_hal/i2c/index.html#flushing

### 3. I2c
I2C Events Reference
- ST: Start condition
- SR: Repeated Start condition  
- SP: Stop condition
- SAD+W: Slave Address + Write bit
- SAD+R: Slave Address + Read bit
- ACK: Acknowledge
- NAK: Not Acknowledge

Requirement 1. Address mode independence
The behaviour should not differ based on if it is using SevenBitAddress or TenBitAddress

Requirement 2. Read
1. The transaction is performed as described in I2C Events
2. The data is stored in the given buffer
3. Should end with a NAK + SP
4. Controller should send ACK for all bytes except last

Requirement 3. Zero Length Write
The implementation should be able to handle zero-length buffers write
and send stop early in that case.

Discussion: https://github.com/rust-embedded/embedded-hal/issues/570#issuecomment-1902829296

Requirement 4. Write
1. The transaction is performed as described in I2C Events
2. The data is written from the given buffer

Requirement 5. Write Read
1. Utilising a single transaction
  1. Issue only one stop 
  2. Use Repeated start between phases
2. Perform the transaction as specified in the I2C Events
3. Should end with a NAK + SP
4. Should handle zero-sized write the same as Requirement 3.

Requirement 6. Error: Address Not Acknowledge
When the target device sends a NAK for the address an error must be returned.
The error kind should be Address but may be Unknown

Requirement 7. Error: Data Not Acknowledge
When the target device sends a NAK for data an error must be returned.
The error kind should be Data but may be Unknown

Requirement 8. Transaction
1. Before executing the first operation an ST is sent automatically. This is followed by SAD+R/W as appropriate.
2. Data from adjacent operations of the same type are sent after each other without an Stop or Repeated Start.
3. Between adjacent operations of a different type an SR and SAD+R/W is sent
4. After executing the last operation an SP is sent automatically 
5. If the last operation is a Read the master does not send an acknowledge for the last byte
6. on empty transaction nothing should happen

Requirement 9. Equivalence to Transaction
For read, write and write_read the operation should match what would be done if given these operations to the transaction API instead.

Requirement 10. No spurious errors
The functions must not return errors during normal bus operations.
- Timeouts during normal operation
- False errors

Requirement 11. No Panic
Should return an error if an operation is not supported rather than panic

---
# Notes
- Multi-master is out of scope
- ErrorKinds Bus, arbitrationloss, Overrun can not be reliably triggered in a generic manner.
- Clock stretching support is implementation-dependent but should be documented
