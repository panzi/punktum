package main

import (
	"fmt"
	"os"
	"io"
	"errors"
	"os/exec"
	"log"
	"strings"
	"github.com/compose-spec/compose-go/v2/dotenv"
)

func main() {
	path := os.Getenv("DOTENV_CONFIG_PATH")
	if path == "" {
		path = ".env"
	}

	replace := false

	i := 1
	for i < len(os.Args) {
		arg := os.Args[i]

		if arg == "--file" || arg == "-f" {
			i += 1
			path = os.Args[i]
		} else if arg == "--replace" || arg == "-r" {
			replace = true
		} else if arg == "--" {
			i += 1
			break
		} else if strings.HasPrefix(arg, "-") {
			log.Fatal("illegal argument: ", arg)
		} else {
			break
		}

		i += 1
	}

	prog := os.Args[i]
	args := os.Args[i + 1:]

	progPath, err := exec.LookPath(prog)
	if err != nil {
		log.Fatal(err)
	}

	cmd := exec.Command(progPath, args...)

	stdout, err := cmd.StdoutPipe()
	if err != nil {
		log.Fatal(err)
	}

	stderr, err := cmd.StderrPipe()
	if err != nil {
		log.Fatal(err)
	}

	if replace {
		environ := os.Environ()
		currentEnv := make(map[string]string)
		for _, pair := range environ {
			parts := strings.SplitN(pair, "=", 2)
			currentEnv[parts[0]] = parts[1]
		}

		env, err := dotenv.GetEnvFromFile(currentEnv, []string{ path })
		if err != nil {
			log.Fatal(err)
		}

		cmdEnv := make([]string, len(env))
		for key, value := range env {
			cmdEnv = append(cmdEnv, fmt.Sprintf("%s=%s", key, value))
		}

		cmd.Env = cmdEnv
	} else {
		err := dotenv.Load(path)
		if err != nil {
			log.Fatal(err)
		}
	}

	if err := cmd.Start(); err != nil {
		log.Fatal(err)
	}

	go io.Copy(os.Stdout, stdout)
	go io.Copy(os.Stderr, stderr)

	if err := cmd.Wait(); err != nil {
		var exitErr *exec.ExitError
		if errors.As(err, &exitErr) {
			os.Exit(exitErr.ExitCode())
		}

		log.Fatal(err)
	}
}
