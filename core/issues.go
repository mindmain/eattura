package core

import "fmt"

func PanicCreateIssuesTemplate() string {
	//TODO: Implement PanicCreateIssuesTemplate
	panic("Create issues template")

}

func panicMissConfig(key, value string) string {
	return fmt.Sprintf("Missing configuration  %s not supported value: `%s`", key, value)
}
