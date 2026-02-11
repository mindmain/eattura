package taxes

import "time"

type Name string

const (
	INPS    Name = "INPS"
	IRPEF   Name = "IRPEF"
	IVA     Name = "IVA"
	IRAP    Name = "IRAP"
	IRES    Name = "IRES"
	Forfait Name = "Forfait"
)

type Tax interface {
	Year() int
	Name() Name
	Percent() float64
	Apply(amount float64) float64
}

type FiscalYear interface {
	Year() int
	Total() float64
	Mounth(time.Month) float64
	Sum(time.Month, time.Month) float64
}
