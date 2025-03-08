import json
import os

from waflib import Context, Task
from waflib.Configure import conf
from waflib.TaskGen import after_method, before_method, feature


def configure(ctx):
    ctx.find_program('cargo', var='CARGO')
    meta = ctx.cmd_and_log([ctx.env.CARGO, 'metadata', '--no-deps'], quiet=Context.STDOUT)
    meta = json.loads(meta)
    ctx.env.CARGO_TARGET_DIR = meta['target_directory']

    manifest = ctx.path.find_node('Cargo.toml')
    if manifest is not None:
        ctx.env.CRATE_MANIFEST = manifest.abspath()
    for package in meta['packages']:
        if package['manifest_path'] == ctx.env.CRATE_MANIFEST:
            ctx.env.CRATE_NAME = package['name']
            for target in package['targets']:
                if target['name'] == package['name'] and 'bin' in target['kind']:
                    break
            else:
                ctx.fatal('Could not find default binary in Cargo.toml.')

    ctx.env.CARGO_PROFILE = 'release' if 'RELEASE' in ctx.env.DEFINES else 'debug'
    ctx.env.RUSTFLAGS = [
        '-C', 'relocation-model=pie',
        '-C', 'link-arg=--build-id=sha1',
        '-C', 'opt-level=s',
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
        plat_dir=plat_dir,
        resources=plat_dir.find_node('app_resources.pbpack'),
    )


@after_method('process_source')
@before_method('generate_memory_usage_report')
@feature('cargo')
def build_cargo_app(task_gen):
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
        '-C', 'target-cpu={}'.format(task_gen.env.RUSTC_CPU),
        '-C', 'link-arg=-T{}/{}'.format(task_gen.path.abspath(), task_gen.ldscript),
    ] + [x for obj in objs for x in ('-C', 'link-arg={}'.format(obj.abspath()))])

    inputs = [
        # No Cargo.lock means we're in a workspace. Search for Cargo.toml
        # instead. TODO: Collect these locations from Cargo metadata in
        # the configure step.
        task_gen.path.find_node('Cargo.lock') or task_gen.path.find_node('Cargo.toml')
    ] + task_gen.path.ant_glob('**/*.rs')
    output = task_gen.path.get_bld().find_or_declare(task_gen.target)
    task_gen.create_task('cargo_build', inputs, output)

class cargo_build(Task.Task):
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
        return self.exec_command(['cp', target_path, self.outputs[0].abspath()])
