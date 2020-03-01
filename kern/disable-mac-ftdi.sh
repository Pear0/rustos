#!/bin/sh

# Disables builtin macOS FTDI drivers until next boot.

sudo kextunload -p -b com.apple.driver.AppleUSBFTDI
sudo kextutil -b com.apple.driver.AppleUSBFTDI -p AppleUSBEFTDI-6010-1

# Disable FTDI supplied driver
sudo kextunload -b com.FTDI.driver.FTDIUSBSerialDriver
