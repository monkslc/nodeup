# Installing
*linux*
`sudo curl -Lo /usr/local/bin/nodeup https://github.com/monkslc/nodeup/releases/download/v0.0.3/nodeup-linux && sudo chmod +x /usr/local/bin/nodeup`

*mac*
`sudo curl -Lo /usr/local/bin/nodeup https://github.com/monkslc/nodeup/releases/download/v0.0.3/nodeup-mac && sudo chmod +x /usr/local/bin/nodeup`

# Setup
Create symlinks for node, npm, and npx that point to nodeup
`nodeup control link`

Verify that everything is properly configured
`nodeup control verify`

Install a node version and set it to the default for the current user
`nodeup versions add --default lts`

Setup complete. Test that it worked by running:
`node -v`

# Usage
## Managing Node Versions *Installing a new node version*
`nodeup versions add 12.18.3`
or
`nodeup versions add lts`

*Listing installed node versions*
`nodeup versions list`

*Removing a node version*
`nodeup versions remove 12.18.3`

## Controlling Directory Overrides
*Adding an override*
`nodeup override add 12.18.3`
or
`nodeup override add --default 12.18.3`
Adding an override will change the version of node that gets run for a directory and all of its descendants. Specifying the `--default` flag will set the default version of node for the current user. That means if no override is set for the current directory or any of its ancestors, nodeup will use the default version specified.

*Viewing which version of node will be run for the current directory*
`nodeup override which`

*Removing an override*
`nodeup override remove`
or
`nodeup override remove --default`
Remove will remove an override for the current working directory only. It will not traverse the file tree to find an override in one of its ancestors.

*Listing all overrides*
`nodeup override list`

*Overriding the version with a file*
Adding a `.nvmrc` file to a directory is the equivalent of setting an override for that directory. An example `.nvmrc` file would look like:
```
12.18.3
```

# Uninstalling
todo!()

# How it works
nodeup creates symlinks for node, npm, and npx that point to the nodeup binary. When nodeup is invoked from one of those symlinks, it determines which binary to run based on the current working directory and the name of the command that was run. This means that nodeup won't use any system resources until it, or one of the symlinks that point to it are called.
