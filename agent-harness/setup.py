from setuptools import find_namespace_packages, setup


setup(
    name="cli-anything-floral-notepaper",
    version="0.1.0",
    description="CLI-Anything harness for Floral Notepaper",
    packages=find_namespace_packages(include=["cli_anything.*"]),
    install_requires=["click>=8.1"],
    extras_require={"test": ["pytest>=8"]},
    entry_points={
        "console_scripts": [
            "cli-anything-floral-notepaper=cli_anything.floral_notepaper.floral_notepaper_cli:main"
        ]
    },
)
