pragma solidity >=0.8.20;

contract Send {
    constructor() payable {
        address payable other = payable(0x888888CfAebbEd5554c3F36BfBD233f822e9455f);
        (bool success, ) = other.call{value: 100}("");
        require(success, "Failed");
    }
}
