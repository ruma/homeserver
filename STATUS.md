# Ruma Status

The following chart shows the status of support for the endpoints in the [client-server API specification](https://matrix.org/docs/spec/client_server/latest.html).
If you're interested in working on an API, take a look at the corresponding tracking issue and leave a comment indicating your interest!

Legend:

:white_check_mark: Supported :construction: Partially supported :no_entry_sign: Not supported

<table>
  <tr>
    <th>Status</th>
    <th>Issue</th>
    <th>API endpoint</th>
  </tr>
  <tr>
    <th align="left" colspan="3">Versions</th>
  </tr>
  <tr>
    <td align="center">:white_check_mark:</td>
    <td></td>
    <td>GET /versions</td>
  </tr>
  <tr>
    <th align="left" colspan="3">Login</th>
  </tr>
  <tr>
    <td align="center">:white_check_mark:</td>
    <td></td>
    <td>POST /login</td>
  </tr>
  <tr>
    <td align="center">:white_check_mark:</td>
    <td></td>
    <td>POST /tokenrefresh</td>
  </tr>
  <tr>
    <td align="center">:white_check_mark:</td>
    <td></td>
    <td>POST /logout</td>
  </tr>
  <tr>
    <th align="left" colspan="3">Account registration and management</th>
  </tr>
  <tr>
    <td align="center">:white_check_mark:</td>
    <td></td>
    <td>POST /register</td>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td><a href="https://github.com/ruma/ruma/issues/80">#80</a></td>
    <td>POST /account/password/email/requestToken</td>
  </tr>
  <tr>
    <td align="center">:white_check_mark:</td>
    <td><a href="https://github.com/ruma/ruma/issues/81">#81</a></td>
    <td>POST /account/deactivate</td>
  </tr>
  <tr>
    <td align="center">:white_check_mark:</td>
    <td></td>
    <td>POST /account/password</td>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td><a href="https://github.com/ruma/ruma/issues/82">#82</a></td>
    <td>POST /register/email/requestToken</td>
  </tr>
  <tr>
    <th align="left" colspan="3">Adding account administrative contact information</th>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td><a href="https://github.com/ruma/ruma/issues/4">#4</a></td>
    <td>POST /account/3pid</td>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td><a href="https://github.com/ruma/ruma/issues/5">#5</a></td>
    <td>GET /account/3pid</td>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td><a href="https://github.com/ruma/ruma/issues/83">#83</a></td>
    <td>POST /account/3pid/email/requestToken</td>
  </tr>
  <tr>
    <th align="left" colspan="3">Filtering</th>
  </tr>
  <tr>
    <td align="center">:white_check_mark:</td>
    <td><a href="https://github.com/ruma/ruma/issues/6">#6</a></td>
    <td>GET /user/:user_id/filter/:filter_id</td>
  </tr>
  <tr>
    <td align="center">:white_check_mark:</td>
    <td><a href="https://github.com/ruma/ruma/issues/7">#7</a></td>
    <td>POST /user/:user_id/filter</td>
  </tr>
  <tr>
    <th align="left" colspan="3">Syncing events</th>
  </tr>
  <tr>
    <td align="center">:construction:</td>
    <td><a href="https://github.com/ruma/ruma/issues/8">#8</a></td>
    <td>GET /sync</td>
  </tr>
  <tr>
    <th align="left" colspan="3">Getting events for a room</th>
  </tr>
  <tr>
    <td align="center">:white_check_mark:</td>
    <td><a href="https://github.com/ruma/ruma/issues/9">#9</a></td>
    <td>GET /rooms/:room_id/state</td>
  </tr>
  <tr>
    <td align="center">:white_check_mark:</td>
    <td><a href="https://github.com/ruma/ruma/issues/10">#10</a></td>
    <td>GET /rooms/:room_id/members</td>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td><a href="https://github.com/ruma/ruma/issues/11">#11</a></td>
    <td>GET /rooms/:room_id/state/:event_type/:state_key</td>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td><a href="https://github.com/ruma/ruma/issues/12">#12</a></td>
    <td>GET /rooms/:room_id/state/:event_type</td>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td><a href="https://github.com/ruma/ruma/issues/13">#13</a></td>
    <td>GET /rooms/:room_id/messages</td>
  </tr>
  <tr>
    <th align="left" colspan="3">Sending events to a room</th>
  </tr>
  <tr>
    <td align="center">:construction:</td>
    <td><a href="https://github.com/ruma/ruma/issues/14">#14</a></td>
    <td>PUT /rooms/:room_id/state/:event_type</td>
  </tr>
  <tr>
    <td align="center">:construction:</td>
    <td><a href="https://github.com/ruma/ruma/issues/15">#15</a></td>
    <td>PUT /rooms/:room_id/state/:event_type/:state_key</td>
  </tr>
  <tr>
    <td align="center">:construction:</td>
    <td><a href="https://github.com/ruma/ruma/issues/16">#16</a></td>
    <td>PUT /rooms/:room_id/send/:event_type/:transaction_id</td>
  </tr>
  <tr>
    <th align="left" colspan="3">Redactions</th>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td><a href="https://github.com/ruma/ruma/issues/17">#17</a></td>
    <td>PUT /rooms/:room_id/redact/:event_id/:transaction_id</td>
  </tr>
  <tr>
    <th align="left" colspan="3">Room creation</th>
  </tr>
  <tr>
    <td align="center">:construction:</td>
    <td><a href="https://github.com/ruma/ruma/issues/18">#18</a></td>
    <td>POST /createRoom</td>
  </tr>
  <tr>
    <th align="left" colspan="3">Room aliases</th>
  </tr>
  <tr>
    <td align="center">:white_check_mark:</td>
    <td><a href="https://github.com/ruma/ruma/issues/19">#19</a></td>
    <td>PUT /directory/room/:room_alias</td>
  </tr>
  <tr>
    <td align="center">:white_check_mark:</td>
    <td><a href="https://github.com/ruma/ruma/issues/20">#20</a></td>
    <td>DELETE /directory/room/:room_alias</td>
  </tr>
  <tr>
    <td align="center">:white_check_mark:</td>
    <td><a href="https://github.com/ruma/ruma/issues/21">#21</a></td>
    <td>GET /directory/room/:room_alias</td>
  </tr>
  <tr>
    <th align="left" colspan="3">Joining rooms</th>
  </tr>
  <tr>
    <td align="center">:white_check_mark:</td>
    <td><a href="https://github.com/ruma/ruma/issues/22">#22</a></td>
    <td>POST /rooms/:room_id/invite</td>
  </tr>
  <tr>
    <td align="center">:white_check_mark:</td>
    <td><a href="https://github.com/ruma/ruma/issues/23">#23</a></td>
    <td>POST /join/:room_id_or_alias</td>
  </tr>
  <tr>
    <td align="center">:white_check_mark:</td>
    <td><a href="https://github.com/ruma/ruma/issues/24">#24</a></td>
    <td>POST /rooms/:room_id/join</td>
  </tr>
  <tr>
    <td align="center">:white_check_mark:</td>
    <td><a href="https://github.com/ruma/ruma/issues/25">#25</a></td>
    <td>POST /rooms/:room_id/kick</td>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td><a href="https://github.com/ruma/ruma/issues/26">#26</a></td>
    <td>POST /rooms/:room_id/unban</td>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td><a href="https://github.com/ruma/ruma/issues/27">#27</a></td>
    <td>POST /rooms/:room_id/ban</td>
  </tr>
  <tr>
    <th align="left" colspan="3">Leaving rooms</th>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td><a href="https://github.com/ruma/ruma/issues/28">#28</a></td>
    <td>POST /rooms/:room_id/forget</td>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td><a href="https://github.com/ruma/ruma/issues/29">#29</a></td>
    <td>POST /rooms/:room_id/leave</td>
  </tr>
  <tr>
    <th align="left" colspan="3">Listing rooms</th>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td><a href="https://github.com/ruma/ruma/issues/30">#30</a></td>
    <td>GET /publicRooms</td>
  </tr>
  <tr>
    <th align="left" colspan="3">Profiles</th>
  </tr>
  <tr>
    <td align="center">:white_check_mark:</td>
    <td><a href="https://github.com/ruma/ruma/issues/31">#31</a></td>
    <td>PUT /profile/:user_id/displayname</td>
  </tr>
  <tr>
    <td align="center">:white_check_mark:</td>
    <td><a href="https://github.com/ruma/ruma/issues/32">#32</a></td>
    <td>GET /profile/:user_id/displayname</td>
  </tr>
  <tr>
    <td align="center">:white_check_mark:</td>
    <td><a href="https://github.com/ruma/ruma/issues/33">#33</a></td>
    <td>PUT /profile/:user_id/avatar_url</td>
  </tr>
  <tr>
    <td align="center">:white_check_mark:</td>
    <td><a href="https://github.com/ruma/ruma/issues/34">#34</a></td>
    <td>GET /profile/:user_id/avatar_url</td>
  </tr>
  <tr>
    <td align="center">:white_check_mark:</td>
    <td><a href="https://github.com/ruma/ruma/issues/35">#35</a></td>
    <td>GET /profile/:user_id</td>
  </tr>
  <tr>
    <th align="left" colspan="3">Voice over IP</th>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td><a href="https://github.com/ruma/ruma/issues/36">#36</a></td>
    <td>GET /voip/turnServer</td>
  </tr>
  <tr>
    <th align="left" colspan="3">Typing notifications</th>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td><a href="https://github.com/ruma/ruma/issues/37">#37</a></td>
    <td>PUT /rooms/:room_id/typing/:user_id</td>
  </tr>
  <tr>
    <th align="left" colspan="3">Receipts</th>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td><a href="https://github.com/ruma/ruma/issues/38">#38</a></td>
    <td>POST /rooms/:room_id/receipt/:receipt_type/:event_id</td>
  </tr>
  <tr>
    <th align="left" colspan="3">Presence</th>
  </tr>
  <tr>
    <td align="center">:construction:</td>
    <td><a href="https://github.com/ruma/ruma/issues/39">#39</a></td>
    <td>PUT /presence/:user_id/status</td>
  </tr>
  <tr>
    <td align="center">:construction:</td>
    <td><a href="https://github.com/ruma/ruma/issues/40">#40</a></td>
    <td>GET /presence/:user_id/status</td>
  </tr>
  <tr>
    <td align="center">:construction:</td>
    <td><a href="https://github.com/ruma/ruma/issues/41">#41</a></td>
    <td>POST /presence/list/:user_id</td>
  </tr>
  <tr>
    <td align="center">:construction:</td>
    <td><a href="https://github.com/ruma/ruma/issues/42">#42</a></td>
    <td>GET /presence/list/:user_id</td>
  </tr>
  <tr>
    <th align="left" colspan="3">Content repository</th>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td><a href="https://github.com/ruma/ruma/issues/43">#43</a></td>
    <td>GET /download/:server_name/:media_id</td>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td><a href="https://github.com/ruma/ruma/issues/44">#44</a></td>
    <td>POST /upload</td>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td><a href="https://github.com/ruma/ruma/issues/45">#45</a></td>
    <td>GET /thumbnail/:server_name/:media_id</td>
  </tr>
  <tr>
    <th align="left" colspan="3">Push notifications</th>
  </tr>
  <tr>
    <td align="center">:white_check_mark:</td>
    <td><a href="https://github.com/ruma/ruma/issues/46">#46</a></td>
    <td>POST /pushers/set</td>
  </tr>
  <tr>
    <td align="center">:white_check_mark:</td>
    <td><a href="https://github.com/ruma/ruma/issues/47">#47</a></td>
    <td>GET /pushers</td>
  </tr>
  <tr>
    <th align="left" colspan="3">Push notification rules</th>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td><a href="https://github.com/ruma/ruma/issues/48">#48</a></td>
    <td>PUT /pushrules/:scope/:kind/:rule_id/enabled</td>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td><a href="https://github.com/ruma/ruma/issues/49">#49</a></td>
    <td>GET /pushrules/:scope/:kind/:rule_id/enabled</td>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td><a href="https://github.com/ruma/ruma/issues/50">#50</a></td>
    <td>PUT /pushrules/:scope/:kind/:rule_id</td>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td><a href="https://github.com/ruma/ruma/issues/51">#51</a></td>
    <td>DELETE /pushrules/:scope/:kind/:rule_id</td>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td><a href="https://github.com/ruma/ruma/issues/52">#52</a></td>
    <td>GET /pushrules/:scope/:kind/:rule_id</td>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td><a href="https://github.com/ruma/ruma/issues/53">#53</a></td>
    <td>GET /pushrules</td>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td><a href="https://github.com/ruma/ruma/issues/54">#54</a></td>
    <td>PUT /pushrules/:scope/:kind/:rule_id/actions</td>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td><a href="https://github.com/ruma/ruma/issues/55">#55</a></td>
    <td>GET /pushrules/:scope/:kind/:rule_id/actions</td>
  </tr>
  <tr>
    <th align="left" colspan="3">Third party invites</th>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td><a href="https://github.com/ruma/ruma/issues/56">#56</a></td>
    <td>POST /rooms/:room_id/invite</td>
  </tr>
  <tr>
    <th align="left" colspan="3">Server side search</th>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td><a href="https://github.com/ruma/ruma/issues/57">#57</a></td>
    <td>POST /search</td>
  </tr>
  <tr>
    <th align="left" colspan="3">Room previews</th>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td><a href="https://github.com/ruma/ruma/issues/58">#58</a></td>
    <td>GET /events</td>
  </tr>
  <tr>
    <th align="left" colspan="3">Room tagging</th>
  </tr>
  <tr>
    <td align="center">:white_check_mark:</td>
    <td><a href="https://github.com/ruma/ruma/issues/59">#59</a></td>
    <td>PUT /user/:user_id/rooms/:room_id/tags/:tag</td>
  </tr>
  <tr>
    <td align="center">:white_check_mark:</td>
    <td><a href="https://github.com/ruma/ruma/issues/60">#60</a></td>
    <td>DELETE /user/:user_id/rooms/:room_id/tags/:tag</td>
  </tr>
  <tr>
    <td align="center">:white_check_mark:</td>
    <td><a href="https://github.com/ruma/ruma/issues/61">#61</a></td>
    <td>GET /user/:user_id/rooms/:room_id/tags</td>
  </tr>
  <tr>
    <th align="left" colspan="3">Client config</th>
  </tr>
  <tr>
    <td align="center">:white_check_mark:</td>
    <td><a href="https://github.com/ruma/ruma/issues/62">#62</a></td>
    <td>PUT /user/:user_id/rooms/:room_id/account_data/:type</td>
  </tr>
  <tr>
    <td align="center">:white_check_mark:</td>
    <td><a href="https://github.com/ruma/ruma/issues/63">#63</a></td>
    <td>PUT /user/:user_id/account_data/:type</td>
  </tr>
  <tr>
    <th align="left" colspan="3">Server administration</th>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td><a href="https://github.com/ruma/ruma/issues/64">#64</a></td>
    <td>GET /admin/whois/:user_id</td>
  </tr>
  <tr>
    <th align="left" colspan="3">Event context</th>
  </tr>
  <tr>
    <td align="center">:no_entry_sign:</td>
    <td><a href="https://github.com/ruma/ruma/issues/65">#65</a></td>
    <td>GET /rooms/:room_id/context/:event_id</td>
  </tr>
</table>
