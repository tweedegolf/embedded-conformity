---
name: New Chip / HAL
about: Suggest to add coverage for a new chip or HAL (e. g. ABC32X040)
title: "[New Chip]"
labels: hal-request
assignees: ''

---

**Which chip(-family) should we add?**
Please add a link to the manufacturer, datasheets, etc.

**If this covers a chip family: Which chips should we test?**
Please list all relevant chips or link to a list of chips.

**Which HALs have support for this chip?**
Please add a link to existing HALs.

**Which interfaces does this chip + HAL support and should be tested?**

* GPIO (input, output, `async` wait)
* UART
* SPI
* I2C
* ...

**Which development boards are available for this chip?**
Prefer boards with an Arduino Uno R3 style header, otherwise indicate that we will need an adapter.
