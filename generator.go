package eattura

import (
	"context"
	"fmt"
	"time"
)

type GeneratorProgressive interface {

	// Return a progressive number and a progressive unique string
	// Progressive number is a value max 10 characters and will be used for the invoice progressive number
	// Progressive unique is a value max 5 characters and will be used for the invoice  file name es. IT12345678901_[00001].xml
	Generate(ctx context.Context) (string, string, error)
}

type defaultGeneratorProgressive struct {
	last int64
}

func (d *defaultGeneratorProgressive) Generate(_ context.Context) (string, string, error) {

	tick := time.Now().Unix()

	if tick == d.last {
		time.Sleep(1 * time.Second)
		tick = time.Now().Unix()
	}

	progNumber, err := getProgressiveNumber(tick)
	if err != nil {
		return "", "", err
	}

	progUnique, err := getProgressiveUnique(tick)
	if err != nil {
		return "", "", err
	}

	return progNumber, progUnique, nil

}

var base62 = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz"

// transform timestamp to base62
func getProgressiveNumber(tick int64) (string, error) {

	if tick == 0 {
		return "", fmt.Errorf("timestamp is zero")
	}

	var result [10]byte = [10]byte{'0', '0', '0', '0', '0', '0', '0', '0', '0', '0'}

	n := tick
	b62 := int64(len(base62))
	index := 9
	for n > 0 {
		result[index] = base62[n%b62]
		n /= b62
		index--
		if index < 0 {
			break
		}
	}

	return string(result[index+1:]), nil

}

// transform timestamp to base62 and cut to resize to 5 characters
func getProgressiveUnique(tick int64) (string, error) {

	if tick == 0 {
		return "", fmt.Errorf("timestamp is zero")
	}

	var result [5]byte = [5]byte{'0', '0', '0', '0', '0'}
	n := tick
	b62 := int64(len(base62))
	index := 4

	for n > 0 {
		result[index] = base62[n%b62]
		n /= b62
		index--
		if index < 0 {
			break
		}
	}

	return string(result[index+1:]), nil

}
