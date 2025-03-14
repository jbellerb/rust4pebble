import json
import os

from waflib import Context, Task
from waflib.Configure import conf
from waflib.TaskGen import after_method, before_method, feature


def configure(ctx):
    ctx.find_program('cargo', var='CARGO')
    meta = ctx.cmd_and_log(
        [ctx.env.CARGO, 'metadata', '--no-deps', '--format-version', '1'],
        quiet=Context.STDOUT,
    )
    meta = json.loads(meta)

    def path_to_relative(path):
        return ctx.root.find_node(path.encode('ascii', 'ignore')).path_from(ctx.path)

    ctx.env.CARGO_TARGET_DIR = path_to_relative(meta['target_directory'])
    ctx.env.CARGO_WORKSPACE_ROOT = path_to_relative(meta['workspace_root'])

    manifest = ctx.path.find_node('Cargo.toml')
    if manifest is not None:
        ctx.env.CRATE_MANIFEST = manifest.path_from(ctx.path)
    for package in meta['packages']:
        if path_to_relative(package['manifest_path']) == ctx.env.CRATE_MANIFEST:
            for target in package['targets']:
                if target['name'] == package['name'] and 'bin' in target['kind']:
                    break
            else:
                ctx.fatal('Could not find default binary in Cargo.toml.')
            ctx.env.CRATE_NAME = package['name']
            break
    else:
        ctx.fatal('Could not find crate.')

    ctx.env.CARGO_PROFILE = 'release' if 'RELEASE' in ctx.env.DEFINES else 'debug'
    ctx.env.RUSTFLAGS = [
        '-C', 'relocation-model=pie',
        '-C', 'codegen-units=1',
        '-C', 'link-arg=--gc-sections',
        '-C', 'link-arg=--build-id=sha1',
        '-C', 'link-arg=--emit-relocs',
        '-C', 'opt-level={}'.format('z' if ctx.env.CARGO_PROFILE == 'release' else '0'),
        '-C', 'debuginfo=2',
    ]


@conf
def cargo_build(self, target=[], bin_type=[]):
    if bin_type != 'app':
        self.fatal('Currently the only supported bin_type is "app".')

    plat_dir = self.path.find_or_declare(self.env.BUILD_DIR)
    return self(
        target=target,
        features='c pebble_cprogram cargo memory_usage',
        bin_type=bin_type,
        app=target,
        inputs=self.path.ant_glob('src/**/*.rs'),
        resources=plat_dir.make_node('app_resources.pbpack'),
    )


@after_method('process_source')
@feature('cargo')
def add_cargo_package(task_gen):
    if task_gen.env.PLATFORM_NAME == 'aplite':
        task_gen.env.RUSTC_TARGET = 'thumbv7m-none-eabi'
        task_gen.env.RUSTC_CPU = 'cortex-m3'
    elif task_gen.env.PLATFORM_NAME in ['basalt', 'chalk', 'diorite']:
        task_gen.env.RUSTC_TARGET = 'thumbv7em-none-eabi'
        task_gen.env.RUSTC_CPU = 'cortex-m4'
    else:
        task_gen.fatal('Unrecognized platform: {}'.format(task_gen.env.PLATFORM_NAME))

    objs = [task.outputs[0] for task in task_gen.compiled_tasks]
    task_gen.env.append_value('RUSTFLAGS', [
        '--cfg=pebble_sdk_platform="{}"'.format(task_gen.env.PLATFORM_NAME),
    ] + [x for obj in objs for x in ('-C', 'link-arg={}'.format(obj.abspath()))])

    inputs = []
    if task_gen.env.CRATE_MANIFEST != task_gen.env.CARGO_WORKSPACE_ROOT + '/Cargo.toml':
        inputs.append(task_gen.path.find_node(task_gen.env.CRATE_MANIFEST))
    inputs.extend([
        task_gen.path.find_node(task_gen.env.CARGO_WORKSPACE_ROOT + '/Cargo.toml'),
        task_gen.path.find_node(task_gen.env.CARGO_WORKSPACE_ROOT + '/Cargo.lock'),
    ] + task_gen.inputs)
    output = task_gen.path.get_bld().find_or_declare(task_gen.target)

    task_gen.cargo_task = task_gen.create_task('cargo_build', inputs, output)
    task_gen.cargo_task.inputs.extend(objs)


@after_method('add_cargo_package')
@before_method('generate_memory_usage_report')
@feature('cargo')
def add_cargo_ldscript(task_gen):
    ldscript_node = task_gen.path.make_node(task_gen.ldscript)
    task_gen.env.append_value('RUSTFLAGS', [
        '-C', 'link-arg=-T{}'.format(ldscript_node.abspath()),
    ])
    task_gen.cargo_task.dep_nodes.append(ldscript_node)


class cargo_build(Task.Task):
    color = 'YELLOW'

    def run(self):
        task_gen = self.generator
        cargo_cmd = [
            self.env.CARGO, 'build',
            '--package', self.env.CRATE_NAME,
            '--target', self.env.RUSTC_TARGET,
        ]
        if self.env.CARGO_PROFILE == 'release':
            cargo_cmd.append('--release')

        env = os.environ.copy()
        env['RUSTFLAGS'] = ' '.join(self.env.RUSTFLAGS)
        ret = self.exec_command(cargo_cmd, cwd=task_gen.path.abspath(), env=env)
        if ret != 0:
            return ret

        target_path = '{}/{}/{}/{}'.format(
            self.env.CARGO_TARGET_DIR,
            self.env.RUSTC_TARGET,
            self.env.CARGO_PROFILE,
            self.env.CRATE_NAME,
        )
        target = task_gen.path.find_node(target_path)
        return self.exec_command(['cp', target.abspath(), self.outputs[0].abspath()])
