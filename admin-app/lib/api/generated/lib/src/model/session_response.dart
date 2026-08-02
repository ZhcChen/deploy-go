//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:deploy_go_api_client/src/model/user_identity.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'session_response.g.dart';

/// SessionResponse
///
/// Properties:
/// * [csrfToken]
/// * [user]
@BuiltValue()
abstract class SessionResponse implements Built<SessionResponse, SessionResponseBuilder> {
  @BuiltValueField(wireName: r'csrf_token')
  String get csrfToken;

  @BuiltValueField(wireName: r'user')
  UserIdentity get user;

  SessionResponse._();

  factory SessionResponse([void updates(SessionResponseBuilder b)]) = _$SessionResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(SessionResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<SessionResponse> get serializer => _$SessionResponseSerializer();
}

class _$SessionResponseSerializer implements PrimitiveSerializer<SessionResponse> {
  @override
  final Iterable<Type> types = const [SessionResponse, _$SessionResponse];

  @override
  final String wireName = r'SessionResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    SessionResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'csrf_token';
    yield serializers.serialize(
      object.csrfToken,
      specifiedType: const FullType(String),
    );
    yield r'user';
    yield serializers.serialize(
      object.user,
      specifiedType: const FullType(UserIdentity),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    SessionResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required SessionResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'csrf_token':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.csrfToken = valueDes;
          break;
        case r'user':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(UserIdentity),
          ) as UserIdentity;
          result.user.replace(valueDes);
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  SessionResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = SessionResponseBuilder();
    final serializedList = (serialized as Iterable<Object?>).toList();
    final unhandled = <Object?>[];
    _deserializeProperties(
      serializers,
      serialized,
      specifiedType: specifiedType,
      serializedList: serializedList,
      unhandled: unhandled,
      result: result,
    );
    return result.build();
  }
}
