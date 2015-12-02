#!/bin/bash
#Support
sudo apt-get install libsdl2-dev libsdl2-image-dev libsdl2-ttf-dev libssl-dev p7zip
#Emulators
sudo apt-get install desmume
sudo apt-get install dolphin-emu
sudo apt-get install mupen64plus
sudo apt-get install nestopia
sudo apt-get install pcsx2
sudo apt-get install visualboyadvance
sudo apt-get install zsnes:i386

wget http://www.lxdream.org/count.php?file=lxdream_0.9.1_amd64.deb -O lxdream.deb
sudo dpkg -i lxdream.deb
sudo apt-get install -f
