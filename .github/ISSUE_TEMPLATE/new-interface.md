---
name: New Interface to Test
about: Suggest to add a tests for a new interface (e. g. I2C, UART, SPI)
title: "[New Interface]"
labels: test-request
assignees: ''

---

**Which documentation or standards exist for this interface?**
e. g. application notes, datasheets, standards, test procedures

**Is there a `trait` describing this interface?**
e. g. in `embedded-hal`

**Which HALs and chips have support for this interface?**
Please link to the matching documentation pages.

**Can this interface be simulated with PIO or bit-banging?**
If so, are there any examples? If not, what is needed additionally?

**Does this interface require special hardware, apart of normal GPIO connections?**
e. g. an external PHY, or low pass filter

**Which cases should the new tests cover?**
e. g. Write/Read transaction, Error cases, Drop behavior, `async` cancelation
