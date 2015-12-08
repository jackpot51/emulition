#!/bin/bash
#Build
sudo apt-get install libsdl2-dev libsdl2-image-dev libsdl2-ttf-dev libssl-dev
#Runtime
sudo apt-get install p7zip
#Emulators
##DS
sudo apt-get install desmume
##Gamecube, Wii
sudo apt-get install dolphin-emu
##N64
sudo apt-get install mupen64plus
##PS2
sudo apt-get install pcsx2
##GBA, NES, SNES, Genesis, PSX
wget http://archive.getdeb.net/ubuntu/rpool/games/m/mednafen/mednafen_0.9.38.7-1~getdeb1_amd64.deb -O mednafen.deb
sudo dpkg -i mednafen.deb
sudo apt-get install -f
##Dreamcast
wget http://www.lxdream.org/count.php?file=lxdream_0.9.1_amd64.deb -O lxdream.deb
sudo dpkg -i lxdream.deb
sudo apt-get install -f

