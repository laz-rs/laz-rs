import pylas
import sys

las = pylas.read(sys.argv[1])
las.points = las.points[int(sys.argv[2]):]
las.write(sys.argv[3])