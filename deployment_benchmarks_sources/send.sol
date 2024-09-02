pragma solidity >=0.8.20;

contract Send {
    constructor() {
        // address payable self = payable(msg.sender);
        address payable other = payable(0x0000000000000000000000000000000000000000);
        other.call{value: 99999999999999999393211750000000};
    }
}
