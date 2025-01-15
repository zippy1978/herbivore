import asyncio
import logging
import pyfiglet

import click
from herbivore.websocket_client import WebSocketClient

@click.command()
@click.option("--userid", type=str, required=True)
@click.option("--nodetype", type=click.Choice(['1x', '1.25x', '2x']), default='1x')
def cli(userid: str, nodetype: str):
    logger = logging.getLogger(__name__)
    logger.setLevel(logging.INFO)
    console_handler = logging.StreamHandler()
    console_handler.setLevel(logging.INFO)
    formatter = logging.Formatter(
        '%(asctime)s - %(name)s - ' + 
        click.style('%(levelname)s', fg='yellow' if '%(levelname)s' == 'WARNING' 
                   else 'red' if '%(levelname)s' == 'ERROR'
                   else 'green' if '%(levelname)s' == 'INFO'
                   else 'white') +
        ' - %(message)s'
    )
    console_handler.setFormatter(formatter)
    logger.addHandler(console_handler)
    
    client = WebSocketClient(userid, nodetype, logger)
    
    print(click.style(pyfiglet.figlet_format("Herbivore"), fg="green"))
    
    asyncio.run(client.start())

if __name__ == "__main__":
    cli()