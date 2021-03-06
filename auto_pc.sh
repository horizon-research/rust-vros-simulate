#!/usr/bin/env bash

for LEVEL2 in 1440
do
    for SCREEN in 1440
    do
        for THRESHOLD in 0.80 0.84 0.88 0.90 0.92 0.94 0.96 0.98 1.0
        do
            cargo run user_viewport_result/Elephant-training-2bpICIClAIg tracedump/elephant.txt object_as_cluster/elephant.json ${THRESHOLD} 20 1440 1440 1440 1440 power O0 >> elephant_power.txt &
        done
        wait

        for THRESHOLD in 0.80 0.84 0.88 0.90 0.92 0.94 0.96 0.98 1.0
        do
            cargo run user_viewport_result/Rhino-training-7IWp875pCxQ tracedump/rhinos.txt object_as_cluster/rhinos.json ${THRESHOLD} 20 1440 1440 1440 1440 power O0 >> rhino_power.txt &
        done
        wait

        for THRESHOLD in 0.80 0.84 0.88 0.90 0.92 0.94 0.96 0.98 1.0
        do
            cargo run user_viewport_result/Rollercoaster-8lsB-P8nGSM tracedump/roller.txt object_as_cluster/roller.json ${THRESHOLD} 20 1440 1440 1440 1440 power O0 >> roller_power.txt &
        done
        wait

        for THRESHOLD in 0.80 0.84 0.88 0.90 0.92 0.94 0.96 0.98 1.0
        do
            cargo run user_viewport_result/Timelapse-CIw8R8thnm8 tracedump/nyc.txt object_as_cluster/nyc.json ${THRESHOLD} 20 1440 1440 1440 1440 power O0 >> nyc_power.txt &
        done
        wait

        for THRESHOLD in 0.80 0.84 0.88 0.90 0.92 0.94 0.96 0.98 1.0
        do
            cargo run user_viewport_result/Paris-sJxiPiAaB4k tracedump/paris.txt object_as_cluster/paris.json ${THRESHOLD} 20 1440 1440 1440 1440 power O0 >> paris_power.txt &
        done
        wait
    done
    wait
done

#for LEVEL2 in 2200 2300 2400 2500 2600 2700 2800 2900 3000 3100 3200 3300 3400
#do
#    for SCREEN in 1200 1300 1400 1500 1600 1700 1800 1900 2000
#    do
#        echo SCREEN: ${SCREEN} LEVEL2: ${LEVEL2}
#        cargo run user_viewport_result/Elephant-training-2bpICIClAIg tracedump/elephant.txt object_as_cluster/elephant.json 0.96 20 ${SCREEN} ${SCREEN} ${LEVEL2} 2160 power O0 >> elephant_power.txt &
#        cargo run user_viewport_result/Rhino-training-7IWp875pCxQ tracedump/rhinos.txt object_as_cluster/rhinos.json 0.96 20 ${SCREEN} ${SCREEN} ${LEVEL2} 2160 power O0 >> rhino_power.txt &
#        cargo run user_viewport_result/Rollercoaster-8lsB-P8nGSM tracedump/roller.txt object_as_cluster/roller.json 0.96 20 ${SCREEN} ${SCREEN} ${LEVEL2} 2160 power O0 >> roller_power.txt &
#        cargo run user_viewport_result/Timelapse-CIw8R8thnm8 tracedump/nyc.txt object_as_cluster/nyc.json 0.96 20 ${SCREEN} ${SCREEN} ${LEVEL2} 2160 power O0 >> nyc_power.txt &
#        cargo run user_viewport_result/Paris-sJxiPiAaB4k tracedump/paris.txt object_as_cluster/paris.json 0.96 20 ${SCREEN} ${SCREEN} ${LEVEL2} 2160 power O0 >> paris_power.txt &
#    done
#    wait
#done
