from __future__ import print_function


def mt(count, pattern, tight=False, prefix='', paren=True):
    if prefix and count:
        prefix += ',' if tight else ', '

    val = prefix + (',' if tight else ', ').join([pattern.format(i, i+1) for i in range(count)])
    return '(' + val + ')' if paren else val


def emit_func(has_err, num_outputs):

    print('#[macro_export]')
    print('macro_rules! do_syscall{}{} '.format(num_outputs, 'r' if has_err else '') + '{')

    for num in range(8):
        print('    {} => '.format(mt(num, '$i{}:expr', tight=True, prefix='$sys:expr')) + '({')
        if num:
            print('        let {}: {} = {};'.format(mt(num, 'i{}'), mt(num, 'u64'), mt(num, '$i{}')))
        if num_outputs:
            print('        {}'.format(' '.join(['let mut o{}: u64;'.format(i) for i in range(num_outputs)])))
        if has_err:
            print('        let mut ecode: u64;')

        offset = num_outputs
        if has_err:
            offset += 1

        print('        asm!(')
        print('            "svc ${}"'.format(offset))

        suffix = ''
        if has_err:
            suffix = ', "={x7}"(ecode)'
        print('            : {}'.format(mt(num_outputs, '"={{x{0}}}"(o{0})', paren=False) + suffix))

        print('            : {}'.format(mt(num, '"{{x{0}}}"(i{0})', paren=False, prefix='"i"($sys)')))

        print('            : "memory"')
        print('            : "volatile" );')

        if has_err:
            print('        err_or!(ecode, {})'.format(mt(num_outputs, 'o{}')))
        else:
            print('        ' + mt(num_outputs, 'o{}'))

        print('    });')
    print('}')


def emit_file():
    print("#![allow(unused_macros)]\n")

    for has_err in [False, True]:
        for num_outputs in range(8):
            emit_func(has_err, num_outputs)


import sys
sys.stdout = open('src/syscall_macros.rs', 'w')

emit_file()
