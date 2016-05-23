import os
import time
import sys

times = 5

if len(sys.argv) > 2:
    times = int(sys.argv[2])

goosetimes = []
goosedownloaded = 0
for i in range(times):
    print "goose number: " + `i`
    start = time.time()
    os.system("./silly_goose " + str(sys.argv[1]) + " dump > /dev/null 2>&1")
    goosetimes.append(time.time() - start)
    goosedownloaded += os.stat('dump').st_size
    os.remove("dump")

wgettimes = []
wgetdownloaded = 0
for i in range(times):
    print "wget number: " + `i`
    start = time.time()
    os.system("wget -O dump " + str(sys.argv[1]) + " > /dev/null 2>&1")
    wgettimes.append(time.time() - start)
    wgetdownloaded += os.stat('dump').st_size
    os.remove("dump")

print "Goosetimes: \n"
for time in goosetimes:
    print `time`

print "wgettimes: \n"
for time in wgettimes:
    print `time`

print "average goose time: " + `(sum(goosetimes, 0.0) / len(goosetimes))`
print "download speed: " + `((goosedownloaded / sum(goosetimes, 0.0)) / 1000) / 1000` + " MB/s"
print "average wget time: " + `(sum(wgettimes, 0.0) / len(wgettimes))` 
print "download speed: " + `((goosedownloaded / sum(wgettimes, 0.0)) / 1000) / 1000` + " MB/s"