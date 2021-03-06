# R-exec
The tool allows one to run executables remotely.
The command is posted by the HTTP POST method containing a JSON-based description.
The stdout of the remote process is streamed to the HTTP response. The HTTP connection is kept open until
the remote process is running.
Closing a connection will stop a remote process.

## Architecture

The tool consists of the Web API module, providing a web interface, 
the Broker module, which initiates execution of child processes, 
and two types of asynchronous tasks, which are the task processing the Http reply,
and the asynchronous task, which consumes data provided by the child process.
The modules communicate to each other by means of channels. 

The Http request is received by the web api module in a form of the POST request
with a JSON coded body. The web server spawns a Response task per request.

After the body is parsed successfully, the start command is sent 
to the Broker. The broker spawns a task for the child process.
The Child process is started by the Operating System.
Successful confirmation is sent to the Response task.
The child tasks starts sending lines generated by the Child
process to the Response task.

As soon as the child process finishes, the stdout channel is being closed 
and the response task finishes. The Exit ProcessStatusMessage is sent to the broker.
The Broker keeps track of running processes.
Only one copy of the same process can run. The unique identifier of the process
is defined by the alias field in the request.

If the calling process breaks a connection, the child process is killed.
Thus, to stop the remote process it is sufficient to close the connection
to the web api module.
    
If graceful shutdown is required, the Shutdown message can be sent to the
Broker module. 

### Block diagram
The basic block diagram is shown below. 

```
                                    
    1     ---------------                         ---------------
 Request  |   Web Api   |      Shutdown           |   Broker    |
--------->|   Module    |------------------------>|  for child  |
          |             | 3 ProcessCreateMessage  |  processes  |
          |             |         --------------->|             |
          ---------------        |                ---------------
                ||    2          |               4      ||   /|\
                ||  spawn        |             spawn    ||    |         10
                ||  task         |             task     ||    | ProcessStatusMessage
               \||/              |                     \||/   |    (child finished)
                \/               |                      \/    |
          ---------------        |                 -----------------            --------------
          |             |_______/                  |               |     5      |            |
    9     |             |   6 StartConfirmation    |               |-----------\|   Child    |
 Response |   Response  |<-------------------------| Child Process |-----------/|  Process   |
<---------|    Task     |      9    Stdout         |     Task      | 7 Child    | Operating  |
          |             |<-------------------------|               |<-----------|  System    |
          ---------------                          -----------------  stdout    --------------

```
  

## Command Line Options

```
    -i, --ip <IP_ADDRESS>              Sets the IP address to bind to. [default: 0.0.0.0]
    -p, --port <IP_PORT>               Sets the IP port to bind to. [default: 8910]
        --status-size <STATUS_SIZE>    Sets the size of the status message channel [default: 8]
        --stdout-size <STDOUT_SIZE>    Sets the size of the stdout channel [default: 8]
    -v                                 Sets the level of verbosity (default is Info, -v is Debug, -vv is Trace )
```




## Request format
The request has a json format.

## API url
The API url is `/process`.
All other url's are not supported and will return a 501 Not Implemented code.

### curl test command
```
curl http://localhost:8910/process \
-H "Content-Type: application/json" \
-X POST \
-d '{"alias":"du", "cmd":"du", "args":["-h", "-c"] }' \
-v
```

### alias
Is a unique short name of the command.
If process with the same alias is already running,
the error wil be returned.

### cmd
A fully qualified or a short name of the command.
If a short name is provided it is up to the operating system
to find the executable in the PATH variable.

### cwd (optional)
A working directory for the command.
If not set the `.` will be used as a current directory for the command.
 

### args (optional)
A list of arguments to be passed to the command

### envs (optional)
A list of environment variables to be set before execution of the command.

### Example of the command message 
This command will run a shell, which will execute 
the ls command with options -rtl.
The request is equal to: `sh -c "ls -rtl"` 

```
{
    "alias" : "do_ls",
    "cmd": "sh",
    "args": [
        "-c",
        "ls -rtl"
    ],
    "cwd": ".",
    "envs": {
        "PATH": "/bin",
        "SECRET_KEY": "QWE_YUI_345_GHJ_789"
    }
}
```

## Http Return codes
### 500 Internal Server Error
Is returned if any internal error occur in the executor code itself.
The response will contain an error text if any.

### 501 Not Implemented
Is returned if any path except the api end point was requested.
The response will contain an error text if any.

### 404 Non Found
Is returned if the process fail to start for any reason,
and the stdout was not created.
The response will contain an error text if any.

### 409 Conflict
Is returned if the command with equal alias is already running.

