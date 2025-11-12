# Cloned Repositories Manifest

This file tracks the Redox OS repositories cloned for hypervisor development.

## Location

All repositories are cloned in: `/home/user/ockham/redox-repos/`

## Cloned Repositories

### 1. redox
- **URL**: https://github.com/nickbetteridge/redox
- **Purpose**: Main Redox OS build system
- **Status**: Clean working directory

### 2. bootloader
- **URL**: https://github.com/nickbetteridge/bootloader
- **Purpose**: Redox bootloader
- **Status**: Clean working directory

### 3. kernel
- **URL**: https://github.com/nickbetteridge/kernel
- **Purpose**: Redox kernel (contains hypervisor module)
- **Branch**: hypervisor/initial-structure
- **Status**: ⚠️ Has local commits - needs push
- **Local Commit**: 305c6021 - "Add initial type-1 hypervisor module structure"
- **Changes**: Added src/hypervisor/ directory with 14 module files

### 4. drivers
- **URL**: https://github.com/nickbetteridge/drivers
- **Purpose**: Redox device drivers
- **Status**: Clean working directory

### 5. relibc
- **URL**: https://github.com/nickbetteridge/relibc
- **Purpose**: Redox C library
- **Status**: Clean working directory

### 6. redoxfs
- **URL**: https://github.com/nickbetteridge/redoxfs
- **Purpose**: Redox filesystem
- **Status**: Clean working directory

### 7. installer
- **URL**: https://github.com/nickbetteridge/installer
- **Purpose**: Redox installer
- **Status**: Clean working directory

## Repository Management

These repositories are git submodules-style (though not using actual submodules).
They are managed independently and not tracked in the main ockham repository.

### To Update All Repositories

```bash
cd /home/user/ockham/redox-repos
for repo in */; do
    cd "$repo"
    echo "Updating $repo"
    git fetch --all
    cd ..
done
```

### To Push Kernel Changes

The kernel repository has unpushed changes:

```bash
cd /home/user/ockham/redox-repos/kernel
git push -u origin hypervisor/initial-structure
```

## Note

The `redox-repos/` directory is excluded from git tracking via `.gitignore`.
Each repository maintains its own git history independently.
